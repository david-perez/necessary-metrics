#![doc = include_str!("../../readme.md")]
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Expr, Ident, Token, Type, Visibility,
};

mod common;
mod parsing;

#[derive(Debug)]
struct Mod {
    attrs: Vec<Attribute>,
    vis: Visibility,
    mod_token: Token![mod],
    ident: Ident,
    fns: Vec<ItemFn>,
}

#[derive(Debug)]
struct ItemFn {
    attrs: FnAttrs,
    vis: Visibility,
    fn_token: Token![fn],
    ident: Ident,
    args: Punctuated<FnArg, Token![,]>,
    arrow_token: Token![->],
    fn_return_ty: FnReturnTy,
}

#[derive(Debug)]
enum FnReturnTy {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug)]
struct FnAttrs {
    cfg: Vec<Attribute>,
    doc: String,
    description: Option<Expr>,
    unit: Option<Expr>,
}

#[derive(Debug)]
struct FnArg {
    ident: Ident,
    colon_token: Token![:],
    ty: Type,
}

impl ToTokens for FnReturnTy {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ty: Type = match self {
            FnReturnTy::Counter => {
                parse_quote! {
                    ::metrics::Counter
                }
            }
            FnReturnTy::Gauge => {
                parse_quote! {
                    ::metrics::Gauge
                }
            }
            FnReturnTy::Histogram => {
                parse_quote! {
                    ::metrics::Histogram
                }
            }
        };
        ty.to_tokens(tokens)
    }
}

#[proc_macro_attribute]
pub fn necessary_metrics(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mod_ = parse_macro_input!(item as Mod);
    expand_from_parsed(mod_).into()
}

fn expand_from_parsed(mod_: Mod) -> proc_macro2::TokenStream {
    let Mod {
        attrs: mod_attrs,
        vis: mod_vis,
        mod_token,
        ident: mod_name,
        fns,
    } = mod_;

    let metric_fns = fns.into_iter().map(|fn_| expand_metric_fn(fn_));

    let ret: proc_macro2::TokenStream = quote! {
        #(#mod_attrs)* #mod_vis #mod_token #mod_name {
            #(#metric_fns)*
        }
    };

    ret
}

fn expand_metric_fn(fn_: ItemFn) -> proc_macro2::TokenStream {
    let ItemFn {
        attrs:
            FnAttrs {
                cfg,
                doc,
                description,
                unit,
            },
        fn_token,
        vis: fn_vis,
        ident: metric_name_ident,
        args,
        arrow_token,
        fn_return_ty,
    } = fn_;

    let fn_args = args.iter().map(|arg| {
        let FnArg {
            ident: arg_name,
            colon_token,
            ty: arg_ty,
            ..
        } = arg;

        quote! { #arg_name #colon_token #arg_ty }
    });

    let label_cnt = args.len();
    let label_emission = args.iter().map(|arg| {
        let FnArg {
            ident: arg_name, ..
        } = arg;
        let metric_key = arg_name.to_string();

        quote! {
            (#metric_key, #arg_name.to_string())
        }
    });
    let metric_name = metric_name_ident.to_string();
    let (labels_ref, labels_binding) = if label_cnt > 0 {
        (
            quote! { &labels },
            quote! { let labels = [#(#label_emission,)*]; },
        )
    } else {
        (quote! {}, quote! {})
    };
    let metric_emission = match fn_return_ty {
        FnReturnTy::Counter => quote! { ::metrics::counter!(#metric_name, #labels_ref) },
        FnReturnTy::Gauge => quote! { ::metrics::gauge!(#metric_name, #labels_ref) },
        FnReturnTy::Histogram => quote! { ::metrics::histogram!(#metric_name, #labels_ref) },
    };

    // It's kinda odd that the `describe_` macros take in `unit` as the second argument when it is
    // optional, but `description` is mandatory and it is last.
    let description_fn = match description {
        Some(description) => {
            let fn_name = Ident::new(
                &format!("describe_{}", &metric_name),
                metric_name_ident.span(),
            );
            let doc = format!("Describes the metric `{metric_name}`.");

            let description_stmt = match unit {
                Some(unit) => match fn_return_ty {
                    FnReturnTy::Counter => {
                        quote! { ::metrics::describe_counter!(#metric_name, #unit, #description); }
                    }
                    FnReturnTy::Gauge => {
                        quote! { ::metrics::describe_gauge!(#metric_name, #unit, #description); }
                    }
                    FnReturnTy::Histogram => {
                        quote! { ::metrics::describe_histogram!(#metric_name, #unit, #description); }
                    }
                },
                None => match fn_return_ty {
                    FnReturnTy::Counter => {
                        quote! { ::metrics::describe_counter!(#metric_name, #description); }
                    }
                    FnReturnTy::Gauge => {
                        quote! { ::metrics::describe_gauge!(#metric_name,  #description); }
                    }
                    FnReturnTy::Histogram => {
                        quote! { ::metrics::describe_histogram!(#metric_name, #description); }
                    }
                },
            };

            quote! {
                #[doc = #doc]
                #(#cfg)*
                #fn_vis #fn_token #fn_name() {
                    #description_stmt
                }
            }
        }
        None => quote! {},
    };

    quote! {
        #[doc = #doc]
        #(#cfg)*
        #fn_vis #fn_token #metric_name_ident(#(#fn_args,)*) #arrow_token #fn_return_ty {
            #labels_binding
            #metric_emission
        }

        #description_fn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_utils::code_str;
    use pretty_assertions::assert_eq;
    use syn::parse_quote;

    #[test]
    fn expand_empty() {
        let src = parse_quote! {
            #[metrics]
            mod empty {}
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod empty { }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn expand_counter() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                pub fn counter() -> Counter;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = ""]
                pub fn counter() -> ::metrics::Counter {
                    ::metrics::counter!("counter",)
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn expand_gauge() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                pub fn gauge() -> Gauge;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = ""]
                pub fn gauge() -> ::metrics::Gauge {
                    ::metrics::gauge!("gauge",)
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn expand_histogram() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                pub fn histogram() -> Histogram;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = ""]
                pub fn histogram() -> ::metrics::Histogram {
                    ::metrics::histogram!("histogram",)
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn expand_labels() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                pub fn counter(label_key: &str) -> Counter;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = ""]
                pub fn counter(label_key: &str,) -> ::metrics::Counter {
                    let labels = [("label_key", label_key.to_string()),];
                    ::metrics::counter!("counter", &labels)
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn rust_docs_are_forwarded() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                /// Rust docs
                pub fn counter() -> Counter;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = " Rust docs"]
                pub fn counter() -> ::metrics::Counter {
                    ::metrics::counter!("counter",)
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn unit_and_description() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                /// Rust docs
                #[description = "metric description"]
                #[unit = metrics::Unit::Count]
                pub fn histogram() -> Histogram;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = " Rust docs"]
                pub fn histogram() -> ::metrics::Histogram {
                    ::metrics::histogram!("histogram",)
                }

                #[doc = "Describes the metric `histogram`."]
                pub fn describe_histogram() {
                    ::metrics::describe_histogram!(
                        "histogram",
                        metrics::Unit::Count,
                        "metric description"
                    );
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn description_only() {
        let src = parse_quote! {
            #[metrics]
            mod metrics {
                /// Rust docs
                #[description = " expression".trim() ]
                pub fn gauge() -> Gauge;
            }
        };
        let actual = expand_from_parsed(src).to_string();

        let expected = code_str! {
            #[metrics]
            mod metrics {
                #[doc = " Rust docs"]
                pub fn gauge() -> ::metrics::Gauge {
                    ::metrics::gauge!("gauge",)
                }

                #[doc = "Describes the metric `gauge`."]
                pub fn describe_gauge() {
                    ::metrics::describe_gauge!(
                        "gauge",
                        " expression".trim()
                    );
                }
            }
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[should_panic(expected = "Metric description has already been set")]
    fn description_must_only_be_set_once() {
        let _mod: Mod = parse_quote! {
            #[metrics]
            mod metrics {
                #[description = "metric description"]
                #[description = "another metric description"]
                pub fn counter() -> Counter;
            }
        };
    }

    #[test]
    #[should_panic(expected = "Cannot set metric unit without setting metric description")]
    fn cannot_set_unit_without_description() {
        let _mod: Mod = parse_quote! {
            #[metrics]
            mod metrics {
                #[unit = metrics::Unit::Seconds]
                pub fn histogram() -> Histogram;
            }
        };
    }

    #[test]
    #[should_panic(expected = "Metric unit has already been set")]
    fn unit_must_only_be_set_once() {
        let _mod: Mod = parse_quote! {
            #[metrics]
            mod metrics {
                #[description = "metric description"]
                #[unit = metrics::Unit::Seconds]
                #[unit = metrics::Unit::Seconds]
                pub fn gauge() -> Gauge;
            }
        };
    }

    #[test]
    #[should_panic(
        expected = "Only `Counter`, `Gauge`, and `Histogram` (verbatim, no qualified paths) are allowed as return types on functions"
    )]
    fn bad_fn_return_ty() {
        let _mod: Mod = parse_quote! {
            #[metrics]
            mod metrics {
                pub fn counter() -> metrics::Counter;
            }
        };
    }
}
