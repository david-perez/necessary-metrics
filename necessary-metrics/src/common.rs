use syn::spanned::Spanned;

pub(crate) fn error<T>(spanned: &impl Spanned, msg: &'static str) -> syn::Result<T> {
    Err(syn::Error::new(spanned.span(), msg))
}

#[cfg(test)]
pub(crate) mod test_utils {
    macro_rules! code_str {
        ($($t:tt)*) => {{
            // We parse-compile into a `TokenStream` to then convert to a canonical string for
            // comapsion in tests.
            let parsed: proc_macro2::TokenStream = parse_quote!{ $($t)* };

            parsed.to_string()
        }};
    }

    pub(crate) use code_str;
}
