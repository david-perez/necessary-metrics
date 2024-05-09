# `necessary-metrics`

[metrics.rs] is great to emit _metrics of convenience_: "fire and forget"
metrics that you sprinkle throughout your codebase.

```rust
let label_key = std::env::var("TASK_ID").unwrap_or("TASK_ID-unset".to_owned());

metrics::describe_counter!(
    "ram_usage",
    metrics::Unit::Kibibytes,
    "RAM usage of a system task"
);
metrics::counter!("ram_usage", label_key => "build").absolute(69);
```

This already works well if you emit your metric once, in a locally centralized
place, or if you need the flexibility to calculate metric names or labels at
runtime.

This crate hosts a macro that helps with emitting _metrics of necessity_:
metrics whose configuration you know ahead of time and that are essential or
indispensable to your program, for which you'd like a less stringly-typed API
to emit them. You attach it to a Rust module with a bodyless function per
defined metric:

```rust
use necessary_metrics::necessary_metrics;

#[necessary_metrics]
pub mod app_metrics {
    /// Rust docs are separate from the metric description.
    #[description = "task latency"]
    #[unit = metrics::Unit::Count]
    pub fn critical_latency(task_name: &str) -> Gauge;
}

pub fn main() {
    app_metrics::describe_critical_latency();
    app_metrics::critical_latency("build").set(69);
}
```

You can then describe and emit these metrics throughout your codebase without
fear of misspelling their names or forgetting a label.

## Implementation

The macro just desugars to what you would have written, so the functions simply
act as a centralized emission location:

```rust
pub mod app_metrics {
    /// Rust docs are separate from the metric description.
    pub fn critical_latency(task_name: &str) -> metrics::Gauge {
        let labels = [("critical latency", task_name.to_string())];
        metrics::gauge!("critical_latency", &labels)
    }

    pub fn describe_critical_latency() {
        metrics::describe_counter!(
            "critical_latency",
            metrics::Unit::Count,
            "task latency"
        );
    }
}
```

## Acknowledgments

- [Russell Cohen](https://github.com/rcoh).
- Cloudflare's [`foundations`][foundations-crate] crate, specifically the
  [`foundations::telemetry::metrics::metrics`][foundations-crate-metrics-module]
  module from which the idea has been pilfered.

[metrics.rs]: https://metrics.rs/
[foundations-crate]: https://docs.rs/foundations/latest/foundations/index.html
[foundations-crate-metrics-module]: https://docs.rs/foundations/latest/foundations/telemetry/metrics/attr.metrics.html
