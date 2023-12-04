#![allow(clippy::collection_is_never_read)]

use std::iter;

use darling::{
    ast::{Data, Style},
    error::Accumulator,
    Error, FromDeriveInput,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;

use crate::{Field, Variant};

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(command),
    forward_attrs(doc),
    supports(
        struct_named,
        struct_newtype,
        struct_unit,
        enum_named,
        enum_newtype,
        enum_unit,
    )
)]
pub struct Args {
    ident: Ident,
    data: Data<Variant, Field>,
}

impl Args {
    fn to(&self, acc: &mut Accumulator) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => match fields.style {
                Style::Struct => {
                    let fields = fields.fields.iter().map(|field| field.to(acc));

                    quote! {
                        ::serenity::all::CreateCommand::new(name)
                            .description(description)
                            .set_options(::std::vec![#(#fields),*])
                    }
                }
                Style::Tuple => {
                    let field = fields
                        .fields
                        .first()
                        .expect("`Args` should only accept tuple `struct`s with one field");
                    let ty = &field.ty;

                    quote! {
                        <#ty as ::serenity_commands::Command>::to_command(name, description)
                    }
                }
                Style::Unit => {
                    quote! {
                        ::serenity::all::CreateCommand::new(name)
                            .description(description)
                    }
                }
            },
            Data::Enum(variants) => {
                let variants = variants
                    .iter()
                    .map(|variant| variant.to(true, acc))
                    .collect::<Vec<_>>();

                quote! {
                    ::serenity::all::CreateCommand::new(name)
                        .description(description)
                        .set_options(::std::vec![
                            #(#variants),*
                        ])
                }
            }
        };

        quote! {
            fn to_command(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommand {
                #body
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn from(&self) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => {
                let body = match fields.style {
                    Style::Struct => {
                        let arms = fields
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| field.from_match_arm(&idx.into()));

                        let inits = iter::repeat(quote!(::std::option::Option::None))
                            .take(fields.fields.len());

                        let field_init = fields
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| field.from_field_init(&idx.into()));

                        quote! {
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

                            ::std::result::Result::Ok(Self {
                                #(#field_init),*
                            })
                        }
                    }
                    Style::Tuple => {
                        let field = fields
                            .fields
                            .first()
                            .expect("`Args` should only accept tuple `struct`s with one field");
                        let ty = &field.ty;

                        quote! {
                            <#ty as ::serenity_commands::Command>::from_command(options).map(Self)
                        }
                    }
                    Style::Unit => {
                        quote! {
                            ::std::result::Result::Ok(Self)
                        }
                    }
                };

                quote! {
                    #body
                }
            }
            Data::Enum(variants) => {
                let arms = variants.iter().map(Variant::from);

                quote! {
                    let [option] = options else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionCount {
                            got: options.len(),
                            expected: 1,
                        });
                    };

                    match option.name.as_str() {
                        #(#arms,)*
                        unknown => ::std::result::Result::Err(
                            ::serenity_commands::Error::UnknownCommandOption(
                                ::std::borrow::ToOwned::to_owned(unknown)
                            )
                        ),
                    }
                }
            }
        };

        quote! {
            fn from_command(
                options: &[::serenity::all::CommandDataOption],
            ) -> ::serenity_commands::Result<Self> {
                #body
            }
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut acc = Error::accumulator();

        let to = self.to(&mut acc);
        let from = self.from();

        let ident = &self.ident;

        let implementation = quote! {
            #[automatically_derived]
            impl ::serenity_commands::Command for #ident {
                #to

                #from
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
