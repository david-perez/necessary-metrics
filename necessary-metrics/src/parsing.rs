use crate::common::error;
use crate::{FnArg, FnReturnTy};

use super::{FnAttrs, ItemFn, Mod};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{braced, parenthesized, Attribute, Expr, Lit, Token, Type};

const FN_ATTR_ERROR: &str = "Only `#[cfg]` and `#[doc]` are allowed on functions";
const METRIC_KIND_ERROR: &str =
    "Only `Counter`, `Gauge`, and `Histogram` (verbatim, no qualified paths) are allowed as return types on functions";

impl Parse for Mod {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let mod_token = input.parse()?;
        let ident = input.parse()?;
        let content;
        let _brace_token = braced!(content in input);

        let mut fns = Vec::new();
        while !content.is_empty() {
            fns.push(content.parse()?);
        }

        Ok(Self {
            attrs,
            vis,
            mod_token,
            ident,
            fns,
        })
    }
}

impl Parse for ItemFn {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        /// Parse attributes applied to a function item. Remember Rust docs get exposed via
        /// `#[doc]` attributes:
        /// <https://docs.rs/syn/latest/syn/struct.Attribute.html#doc-comments>
        fn parse_attrs(attrs: Vec<Attribute>) -> syn::Result<FnAttrs> {
            let mut cfg = Vec::new();
            let mut doc = "".to_owned();
            let mut description = None;
            let mut unit = None;

            /// Reads as a string the value after the equals sign of an `Attribute` whose path is
            /// of kind `Meta::NameValue`, e.g. an attribute like:
            ///
            ///     #[doc = "value"]
            ///     fn fn_item() { }
            ///
            /// See <https://docs.rs/syn/latest/syn/struct.Attribute.html>.
            fn read_attr_meta_name_value(attr: &Attribute) -> syn::Result<Option<String>> {
                let mnv = attr.meta.require_name_value()?;
                if let Expr::Lit(expr) = &mnv.value {
                    if let Lit::Str(lit_str) = &expr.lit {
                        return Ok(Some(lit_str.value()));
                    }
                }
                return Ok(None);
            }

            /// Reads as an expression the value after the equals sign of an `Attribute` whose path is
            /// of kind `Meta::NameValue`.
            fn read_attr_expr(attr: Attribute) -> syn::Result<Expr> {
                let mnv = attr.meta.require_name_value()?;
                return Ok(mnv.value.clone());
            }

            let mut unit_attr = None;
            for attr in attrs {
                if attr.path().is_ident("cfg") {
                    cfg.push(attr);
                } else if attr.path().is_ident("doc") {
                    if let Some(s) = read_attr_meta_name_value(&attr)? {
                        doc.push_str(&s);
                    }
                } else if attr.path().is_ident("description") {
                    if description.is_some() {
                        return error(&attr, "Metric description has already been set");
                    }
                    description = read_attr_expr(attr).ok();
                } else if attr.path().is_ident("unit") {
                    if unit.is_some() {
                        return error(&attr, "Metric unit has already been set");
                    }
                    unit_attr = Some(attr.clone());
                    unit = read_attr_expr(attr).ok();
                } else {
                    return error(&attr, FN_ATTR_ERROR);
                }
            }

            if unit.is_some() && description.is_none() {
                return error(
                    &unit_attr,
                    "Cannot set metric unit without setting metric description",
                );
            }

            Ok(FnAttrs {
                cfg,
                doc,
                description,
                unit,
            })
        }

        let attrs = parse_attrs(input.call(Attribute::parse_outer)?)?;
        let vis = input.parse()?;
        let fn_token = input.parse()?;
        let ident = input.parse()?;
        let args_content;
        let _paren_token = parenthesized!(args_content in input);
        let mut args = Punctuated::new();

        while !args_content.is_empty() {
            args.push_value(args_content.parse()?);

            if args_content.is_empty() {
                break;
            }

            args.push_punct(args_content.parse()?);
        }

        let arrow_token = input.parse()?;
        let ty = input.parse()?;
        let _semi_token = input.parse::<Token![;]>()?;

        Ok(ItemFn {
            attrs,
            vis,
            fn_token,
            ident,
            args,
            arrow_token,
            fn_return_ty: ty,
        })
    }
}

impl Parse for FnReturnTy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;

        match ty {
            Type::Path(ty) => {
                let ident = ty
                    .path
                    .require_ident()
                    .map_err(|_e| syn::Error::new(Spanned::span(&ty), METRIC_KIND_ERROR))?;

                let kind = match ident.to_string().as_str() {
                    "Counter" => Self::Counter,
                    "Gauge" => Self::Gauge,
                    "Histogram" => Self::Histogram,
                    _ => {
                        return error(&ty, METRIC_KIND_ERROR);
                    }
                };

                Ok(kind)
            }
            _ => {
                return error(&ty, METRIC_KIND_ERROR);
            }
        }
    }
}

impl Parse for FnArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let colon_token = input.parse()?;
        let ty = input.parse()?;

        Ok(Self {
            ident,
            colon_token,
            ty,
        })
    }
}
