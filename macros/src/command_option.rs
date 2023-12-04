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
                        ::serenity::all::CreateCommandOption::new(
                            <Self as ::serenity_commands::CommandOption>::TYPE,
                            name,
                            description,
                        )
                        #(.add_sub_option(#fields))*
                    }
                }
                Style::Tuple => {
                    let field = fields
                        .fields
                        .first()
                        .expect("`Args` should only accept tuple `struct`s with one field");
                    let ty = &field.ty;

                    quote! {
                        <#ty as ::serenity_commands::CommandOption>::to_option(name, description)
                    }
                }
                Style::Unit => {
                    quote! {
                        ::serenity::all::CreateCommandOption::new(
                            <Self as ::serenity_commands::CommandOption>::TYPE,
                            name,
                            description,
                        )
                    }
                }
            },
            Data::Enum(variants) => {
                let variants = variants
                    .iter()
                    .map(|variant| variant.to(false, acc))
                    .collect::<Vec<_>>();

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        <Self as ::serenity_commands::CommandOption>::TYPE,
                        name,
                        description,
                    )
                    #(.add_sub_option(#variants))*
                }
            }
        };

        quote! {
            fn to_option(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommandOption {
                #body
            }
        }
    }

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

                        return quote! {
                            fn from_option(
                                option: ::std::option::Option<&::serenity::all::CommandDataOptionValue>
                            ) -> ::serenity_commands::Result<Self> {
                                <#ty as ::serenity_commands::CommandOption>::from_option(
                                    option
                                ).map(Self)
                            }
                        };
                    }
                    Style::Unit => {
                        quote! {
                            ::std::result::Result::Ok(Self)
                        }
                    }
                };

                quote! {
                    if let ::serenity::all::CommandDataOptionValue::SubCommand(options) = option {
                        #body
                    } else {
                        ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: <Self as ::serenity_commands::CommandOption>::TYPE,
                        })
                    }
                }
            }
            Data::Enum(variants) => {
                let arms = variants.iter().map(Variant::from);

                quote! {
                    let ::serenity::all::CommandDataOptionValue::SubCommandGroup(options) = option else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: <Self as ::serenity_commands::CommandOption>::TYPE,
                        });
                    };

                    let [option] = options.as_slice() else {
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
            fn from_option(
                option: ::std::option::Option<&::serenity::all::CommandDataOptionValue>
            ) -> ::serenity_commands::Result<Self> {
                let option = option.ok_or(::serenity_commands::Error::MissingRequiredCommandOption)?;

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
        let command_option_type = match self.data {
            Data::Struct(_) => quote!(SubCommand),
            Data::Enum(_) => quote!(SubCommandGroup),
        };

        let implementation = quote! {
            #[automatically_derived]
            impl ::serenity_commands::CommandOption for #ident {
                const TYPE: ::serenity::all::CommandOptionType = ::serenity::all::CommandOptionType::#command_option_type;

                #to

                #from
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
