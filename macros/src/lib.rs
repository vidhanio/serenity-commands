#![allow(missing_docs)]
//! Macros for the `serenity_commands` crate.
//!
//! An implementation detail. Do not use directly.

mod basic_option;
mod command;
mod commands;
mod sub_command;
mod sub_command_group;

use std::iter;

use darling::{
    ast::{Fields, Style},
    error::Accumulator,
    util::{Flag, SpannedValue},
    Error, FromDeriveInput, FromField, FromVariant,
};
use heck::ToKebabCase;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
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
        option_name(&self.ident, self.name.as_ref())
    }

    fn create_command(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

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
                    <#ty as ::serenity_commands::Command>::create_command(#name, #description)
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

    fn create_sub_command_or_group(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

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

                quote! {
                    <#ty as ::serenity_commands::SubCommandGroup>::create_option(
                        #name,
                        #description,
                    )
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

    fn create_sub_command(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

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

                quote! {
                    <#ty as ::serenity_commands::SubCommand>::create_option(
                        #name,
                        #description,
                    )
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

    #[allow(clippy::wrong_self_convention)]
    fn from_command_options(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

                quote! {
                    #fold

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
                    <#ty as ::serenity_commands::Command>::from_options(
                        options
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_subcommand_or_group_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

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

                    #fold

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
                    <#ty as ::serenity_commands::SubCommandGroup>::from_value(
                        &option.value
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_subcommand_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

                quote! {
                    let ::serenity::all::CommandDataOption {
                        value: ::serenity::all::CommandDataOptionValue::SubCommand(options),
                        ..
                    } = option else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: value.kind(),
                            expected: ::serenity::all::CommandOptionType::SubCommand,
                        });
                    };

                    #fold

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
                    <#ty as ::serenity_commands::SubCommand>::from_value(
                        &option.value
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

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
    autocomplete: Flag,
}

impl Field {
    fn ident(&self) -> &Ident {
        self.ident
            .as_ref()
            .expect("`Field::ident` should only be called on named fields")
    }

    fn name(&self) -> LitStr {
        option_name(self.ident(), self.name.as_ref())
    }

    fn create_option(&self, acc: &mut Accumulator) -> TokenStream {
        let ident = self.ident();
        let ty = &self.ty;

        let name = self.name();
        let description = documentation_string(&self.attrs, ident, acc);
        let autocomplete = self.autocomplete.is_present();

        quote! {
            <#ty as ::serenity_commands::BasicOption>::create_option(
                #name,
                #description,
            )
            .set_autocomplete(#autocomplete)
        }
    }

    fn from_options(selfs: &[Self]) -> (TokenStream, impl Iterator<Item = TokenStream> + '_) {
        let match_arms = selfs.iter().enumerate().map(|(idx, field)| {
            let idx = Index::from(idx);
            let name = field.name();

            quote! {
                #name => acc.#idx = ::std::option::Option::Some(
                    &option.value
                )
            }
        });

        let inits = iter::repeat(quote!(::std::option::Option::None)).take(selfs.len());

        let field_init = selfs.iter().enumerate().map(|(idx, field)| {
            let ident = field.ident();
            let ty = &field.ty;

            let idx = Index::from(idx);

            quote! {
                #ident: <#ty as ::serenity_commands::BasicOption>::from_value(
                    acc.#idx
                )?
            }
        });

        let fold = quote! {
            let acc = ::std::iter::Iterator::fold(
                options.iter(),
                (#(#inits,)*),
                |mut acc, option| {
                    match option.name.as_str() {
                        #(#match_arms,)*
                        _ => {}
                    }

                    acc
                }
            );
        };

        (fold, field_init)
    }
}

#[proc_macro_derive(Commands, attributes(command))]
pub fn derive_commands(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    commands::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    command::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(SubCommandGroup, attributes(command))]
pub fn derive_sub_command_group(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sub_command_group::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(SubCommand, attributes(command))]
pub fn derive_sub_command(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sub_command::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(BasicOption, attributes(choice))]
pub fn derive_basic_option(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    basic_option::Args::from_derive_input(&parse_macro_input!(tokens))
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

fn option_name(ident: &Ident, s: Option<&SpannedValue<String>>) -> LitStr {
    s.map_or_else(
        || {
            let ident_s = ident.to_string();
            LitStr::new(
                &ident_s
                    .strip_prefix("r#")
                    .unwrap_or(&ident_s)
                    .to_kebab_case(),
                ident.span(),
            )
        },
        |name| LitStr::new(name, name.span()),
    )
}
