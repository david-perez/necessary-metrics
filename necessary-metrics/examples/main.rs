#![allow(warnings)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use metrics::{
    atomics::AtomicU64, counter, Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder,
    SharedString, Unit,
};
use necessary_metrics::necessary_metrics;

#[necessary_metrics]
pub mod metrics_of_necessity {
    /// Rust docs.
    #[description = "metric description"]
    #[unit = metrics::Unit::Milliseconds]
    pub fn critical_counter(label_key: &str) -> Counter;
}

pub mod metrics_of_necessity_desugared {
    /// Metric description.
    pub fn critical_counter(label_key: String) -> metrics::Counter {
        let labels = [("label_key", label_key)];
        metrics::counter!("critical_counter", &labels)
    }

    pub fn describe_critical_counter() {
        metrics::describe_counter!(
            "critical_counter",
            metrics::Unit::Milliseconds,
            "metric description"
        );
    }
}

#[derive(Default)]
struct MetricsOfNecessity {
    /// The metric value.
    critical_counter: Arc<AtomicU64>,
    /// Whether a metric is set or not.
    critical_counter_init: AtomicBool,
}

impl MetricsOfNecessity {
    const CRITICAL_COUNTER: &str = "critical_counter";

    // These two are only generated if specified.
    const CRITICAL_COUNTER_DESCRIPTION: &str = "Metric description";
    const CRITICAL_COUNTER_UNIT: Unit = Unit::Seconds;

    /// Renders a string payload in the target metric format containing all _set_ metrics, using
    /// their corresponding descriptions and units, if specified.
    pub fn render(&self) -> String {
        todo!()
    }

    /// Clears all metrics, rendering a string payload in the target metric format containing all
    /// _set_ metrics, using their corresponding descriptions and units, if specified.
    pub fn drain(&self) -> String {
        todo!()
    }
}

impl Recorder for MetricsOfNecessity {
    // `describe_` methods are no-ops; user should declaratively specify description and unit at compile-time.
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {}
    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {}
    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {}

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        match key.name() {
            Self::CRITICAL_COUNTER => {
                let _was_initialized = self.critical_counter_init.swap(true, Ordering::Acquire);
                return Counter::from_arc(self.critical_counter.clone());
            }
            // User made a mistake (perhaps typoed). Fallibility would be nice, since these metrics
            // are critical.
            _ => Counter::noop(),
        }
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        match key.name() {
            _ => Gauge::noop(),
        }
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        match key.name() {
            _ => Histogram::noop(),
        }
    }
}

fn main() {
    let recorder = MetricsOfNecessity::default();
    metrics::with_local_recorder(&recorder, || {
        counter!(MetricsOfNecessity::CRITICAL_COUNTER).absolute(69);
    });

    assert_eq!(69, recorder.critical_counter.load(Ordering::Acquire));
}
