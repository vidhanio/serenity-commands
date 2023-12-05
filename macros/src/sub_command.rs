use darling::{
    ast::{Data, Style},
    error::Accumulator,
    util::Ignored,
    Error, FromDeriveInput,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, Ident};

use crate::Field;

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(command),
    supports(struct_named, struct_newtype, struct_unit)
)]
pub struct Args {
    ident: Ident,
    generics: Generics,
    data: Data<Ignored, Field>,
}

impl Args {
    fn create_option(&self, acc: &mut Accumulator) -> TokenStream {
        let fields = self.data.as_ref().take_struct().unwrap();

        let body = match fields.style {
            Style::Struct => {
                let options = fields.fields.iter().map(|field| field.create_option(acc));

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        name,
                        description
                    )
                        #(.add_sub_option(#options))*
                }
            }
            Style::Tuple => {
                let field = fields.fields.first().unwrap();
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommand>::create_option(name, description)
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        name,
                        description,
                    )
                }
            }
        };

        quote! {
            fn create_option(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommandOption {
                #body
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_value(&self) -> TokenStream {
        let Data::Struct(fields) = &self.data else {
            unreachable!()
        };

        let body = match fields.style {
            Style::Struct => {
                let (fold, inits) = Field::from_options(&fields.fields);

                quote! {
                    let ::serenity::all::CommandDataOptionValue::SubCommand(options) = value else {
                        return ::std::result::Result::Err(
                            ::serenity_commands::Error::IncorrectCommandOptionType {
                                got: value.kind(),
                                expected: ::serenity::all::CommandOptionType::SubCommand,
                            },
                        );
                    };

                    #fold

                    ::std::result::Result::Ok(Self {
                        #(#inits),*
                    })
                }
            }
            Style::Tuple => {
                let field = fields.fields.first().unwrap();
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommand>::from_value(value)
                        .map(Self)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self)
                }
            }
        };

        quote! {
            fn from_value(
                value: &::serenity::all::CommandDataOptionValue,
            ) -> ::serenity_commands::Result<Self> {
                #body
            }
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut acc = Error::accumulator();

        let ident = &self.ident;

        let create_option = self.create_option(&mut acc);
        let from_value = self.from_value();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SubCommandGroup for #ident #ty_generics #where_clause {
                #create_option

                #from_value
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SubCommand for #ident #ty_generics #where_clause {
                fn create_option(
                    name: impl ::std::convert::Into<::std::string::String>,
                    description: impl ::std::convert::Into<::std::string::String>,
                ) -> ::serenity::all::CreateCommandOption {
                    <Self as ::serenity_commands::SubCommandGroup>::create_option(name, description)
                }

                fn from_value(
                    value: &::serenity::all::CommandDataOptionValue,
                ) -> ::serenity_commands::Result<Self> {
                    <Self as ::serenity_commands::SubCommandGroup>::from_value(value)
                }
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
