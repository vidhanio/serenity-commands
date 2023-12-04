#![allow(missing_docs)]
//! Macros for the `serenity_commands` crate.
//!
//! An implementation detail. Do not use directly.

mod command;
mod command_data;
mod command_option;

use std::iter;

use darling::{
    ast::{Fields, Style},
    error::Accumulator,
    util::SpannedValue,
    Error, FromDeriveInput, FromField, FromVariant,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Expr, ExprLit, Ident, Index, Lit, LitStr, Meta,
    MetaNameValue, Type,
};

#[derive(Debug, FromVariant)]
#[darling(attributes(command), forward_attrs(doc))]
struct Variant {
    ident: Ident,
    fields: Fields<Field>,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,
}

impl Variant {
    fn name(&self) -> LitStr {
        generate_command_name(&self.ident, self.name.as_ref())
    }

    fn to(&self, allow_subcmd_groups: bool, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.to(acc));

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                    #(.add_sub_option(#fields))*
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                let assertion = if allow_subcmd_groups {
                    quote_spanned! {self.ident.span()=>
                        const _: () = {
                            if !::std::matches!(
                                <#ty as ::serenity_commands::CommandOption>::TYPE,
                                ::serenity::all::CommandOptionType::SubCommand | ::serenity::all::CommandOptionType::SubCommandGroup
                            ) {
                                ::std::panic!(::std::concat!(
                                    "newtype variant of a `SubCommandGroup` must be of type `SubCommand` or `SubCommandGroup`",
                                ))
                            }
                        };
                    }
                } else {
                    quote_spanned! {self.ident.span()=>
                        const _: () = {
                            if !::std::matches!(
                                <#ty as ::serenity_commands::CommandOption>::TYPE,
                                ::serenity::all::CommandOptionType::SubCommand,
                            ) {
                                ::std::panic!(::std::concat!(
                                    "newtype variant of a `SubCommandGroup` must be of type `SubCommand`",
                                ))
                            }
                        };
                    }
                };

                quote! {
                    {
                        #assertion
                        <#ty as ::serenity_commands::CommandOption>::to_option(#name, #description)
                    }
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                }
            }
        }
    }

    fn to_data(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.to(acc));

                quote! {
                    ::serenity::all::CreateCommand::new(#name)
                        .description(#description)
                        .set_options(::std::vec![#(#fields),*])
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    {
                        <#ty as ::serenity_commands::Command>::to_command(#name, #description)
                    }
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommand::new(#name)
                        .description(#description)
                }
            }
        }
    }

    fn from(&self) -> TokenStream {
        let ident = &self.ident;
        let name = self.name();

        let match_body = match self.fields.style {
            Style::Struct => {
                let arms = self
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| field.from_match_arm(&idx.into()));

                let inits =
                    iter::repeat(quote!(::std::option::Option::None)).take(self.fields.len());

                let field_init = self
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| field.from_field_init(&idx.into()));

                quote! {
                    let ::serenity::all::CommandDataOption {
                        value: ::serenity::all::CommandDataOptionValue::SubCommand(options),
                        ..
                    } = option else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: ::serenity::all::CommandOptionType::SubCommand,
                        });
                    };

                    let acc = ::std::iter::Iterator::fold(
                        options.iter(),
                        (#(#inits,)*),
                        |mut acc, option| {
                            match option.name.as_str() {
                                #(#arms,)*
                                _ => {}
                            }

                            acc
                        }
                    );

                    ::std::result::Result::Ok(Self::#ident {
                        #(#field_init),*
                    })
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::CommandOption>::from_option(
                        ::std::option::Option::Some(&option.value)
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_data(&self) -> TokenStream {
        let ident = &self.ident;
        let name = self.name();

        let match_body = match self.fields.style {
            Style::Struct => {
                let arms = self
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| field.from_match_arm(&idx.into()));

                let inits =
                    iter::repeat(quote!(::std::option::Option::None)).take(self.fields.len());

                let field_init = self
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| field.from_field_init(&idx.into()));

                quote! {
                    let acc = ::std::iter::Iterator::fold(
                        data.options.iter(),
                        (#(#inits,)*),
                        |mut acc, option| {
                            match option.name.as_str() {
                                #(#arms,)*
                                _ => {}
                            }

                            acc
                        }
                    );

                    ::std::result::Result::Ok(Self::#ident {
                        #(#field_init),*
                    })
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::Command>::from_command(
                        &data.options
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        quote! {
            #name => { #match_body }
        }
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(command), forward_attrs(doc))]
struct Field {
    ident: Option<Ident>,
    ty: Type,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,
}

impl Field {
    fn ident(&self) -> &Ident {
        self.ident
            .as_ref()
            .expect("`Field::ident` should only be called on named fields")
    }

    fn name(&self) -> LitStr {
        generate_command_name(self.ident(), self.name.as_ref())
    }

    fn to(&self, acc: &mut Accumulator) -> TokenStream {
        let ident = self.ident();
        let ty = &self.ty;

        let name = self.name();
        let description = documentation_string(&self.attrs, ident, acc);

        let assertions = quote_spanned! {ident.span()=>
            const _: () = {
                if ::std::matches!(
                    <#ty as ::serenity_commands::CommandOption>::TYPE,
                    ::serenity::all::CommandOptionType::SubCommand
                ) {
                    ::std::panic!(
                        "named field must not be of type `SubCommand`",
                    )
                }

                if ::std::matches!(
                    <#ty as ::serenity_commands::CommandOption>::TYPE,
                    ::serenity::all::CommandOptionType::SubCommandGroup
                ) {
                    ::std::panic!(::std::concat!(
                        "named field must not be of type `SubCommandGroup`",
                    ))
                }
            };
        };

        quote! {
            {
                #assertions
                <#ty as ::serenity_commands::CommandOption>::to_option(
                    #name,
                    #description,
                )
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_match_arm(&self, idx: &Index) -> TokenStream {
        let name = self.name();

        quote! {
            #name => acc.#idx = ::std::option::Option::Some(
                &option.value
            )
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_field_init(&self, idx: &Index) -> TokenStream {
        let ty = &self.ty;

        let ident = self.ident();

        quote! {
            #ident: <#ty as ::serenity_commands::CommandOption>::from_option(
                acc.#idx
            )?
        }
    }
}

#[proc_macro_derive(CommandOption, attributes(command))]
pub fn derive_command_option(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    command_option::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    command::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(CommandData, attributes(command))]
pub fn derive_command_data(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    command_data::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

fn documentation_string(
    attrs: &[Attribute],
    spanned: &impl Spanned,
    acc: &mut Accumulator,
) -> LitStr {
    let res = attrs
        .iter()
        .find_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) if path.is_ident("doc") => Some(LitStr::new(s.value().trim(), s.span())),
            _ => None,
        })
        .ok_or_else(|| {
            Error::custom("missing documentation comment (`///`) to use as description")
                .with_span(spanned)
        });

    acc.handle(res)
        .unwrap_or_else(|| LitStr::new("", Span::call_site()))
}

fn generate_command_name(ident: &Ident, s: Option<&SpannedValue<String>>) -> LitStr {
    s.map_or_else(
        || {
            let ident_s = ident.to_string();
            LitStr::new(
                &ident_s
                    .strip_prefix("r#")
                    .unwrap_or(&ident_s)
                    .to_lowercase(),
                ident.span(),
            )
        },
        |name| LitStr::new(name, name.span()),
    )
}
