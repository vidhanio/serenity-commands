use darling::{error::Accumulator, util::SpannedValue, Error};
use heck::{ToKebabCase, ToTitleCase};
use proc_macro2::Span;
use syn::{spanned::Spanned, Attribute, Expr, ExprLit, Ident, Lit, LitStr, Meta, MetaNameValue};

pub fn documentation_string(
    attrs: &[Attribute],
    spanned: &impl Spanned,
    acc: &mut Accumulator,
) -> LitStr {
    let mut doc_comments = attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) if path.is_ident("doc") => Some((s.span(), s.value())),
            _ => None,
        })
        .peekable();

    let res = if doc_comments.peek().is_none() {
        Err(
            Error::custom("missing documentation comment (`///`) to use as description")
                .with_span(spanned),
        )
    } else {
        let (span, s) = doc_comments.fold(
            (Span::call_site(), String::new()),
            |(span, mut acc), (_, s)| {
                if !acc.is_empty() {
                    acc.push(' ');
                }

                acc.push_str(s.trim());

                (span, acc)
            },
        );

        Ok(LitStr::new(&s, span))
    };

    acc.handle(res)
        .unwrap_or_else(|| LitStr::new("", Span::call_site()))
}

pub trait IdentExt {
    fn to_kebab_case(&self) -> LitStr;
    fn to_title_case(&self) -> LitStr;
    fn autocomplete(&self) -> Ident;
}

impl IdentExt for Ident {
    fn to_kebab_case(&self) -> LitStr {
        let ident_s = self.to_string();
        LitStr::new(
            &ident_s
                .strip_prefix("r#")
                .unwrap_or(&ident_s)
                .to_kebab_case(),
            self.span(),
        )
    }

    fn to_title_case(&self) -> LitStr {
        let ident_s = self.to_string();
        LitStr::new(
            &ident_s
                .strip_prefix("r#")
                .unwrap_or(&ident_s)
                .to_title_case(),
            self.span(),
        )
    }

    fn autocomplete(&self) -> Ident {
        Self::new(&format!("{self}Autocomplete"), self.span())
    }
}

pub fn kebab_name(ident: &Ident, s: Option<&SpannedValue<String>>) -> LitStr {
    s.map_or_else(
        || ident.to_kebab_case(),
        |name| LitStr::new(name, name.span()),
    )
}

pub fn title_name(ident: &Ident, s: Option<&SpannedValue<String>>) -> LitStr {
    s.map_or_else(
        || ident.to_title_case(),
        |name| LitStr::new(name, name.span()),
    )
}
