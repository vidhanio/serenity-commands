use darling::{
    ast::{Data, Style},
    error::Accumulator,
    util::Ignored,
    Error, FromDeriveInput,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, Ident};

use crate::{field::Field, utils::IdentExt, BuilderMethodList};

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(command),
    supports(struct_named, struct_newtype, struct_unit)
)]
pub struct Args {
    ident: Ident,
    generics: Generics,
    data: Data<Ignored, Field>,

    builder: Option<BuilderMethodList>,
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

        let builder_methods = &self.builder;

        quote! {
            fn create_option(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommandOption {
                #body
                #builder_methods
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
                let (fold, field_inits) = Field::from_options(&fields.fields, false);

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
                        #field_inits
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

    fn autocomplete(&self) -> Option<TokenStream> {
        let Data::Struct(fields) = &self.data else {
            unreachable!()
        };

        let autocomplete_ident = self.ident.autocomplete();
        let generics = &self.generics;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let (ty, body) = match fields.style {
            Style::Struct => {
                let variants = Field::autocomplete_variants(&fields.fields);

                if variants.is_empty() {
                    return None;
                }

                let body = Field::from_autocomplete_options(&fields.fields);

                (
                    quote! {
                        pub enum #autocomplete_ident #generics {
                            #variants
                        }
                    },
                    quote! {
                        let ::serenity::all::CommandDataOptionValue::SubCommand(options) = value else {
                            return ::std::result::Result::Err(
                                ::serenity_commands::Error::IncorrectCommandOptionType {
                                    got: value.kind(),
                                    expected: ::serenity::all::CommandOptionType::SubCommand,
                                },
                            );
                        };

                        #body
                    },
                )
            }
            Style::Tuple => {
                let field = fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `struct`s with one field");

                if !field.autocomplete.is_present() {
                    return None;
                }

                let ty = &field.ty;

                (
                    quote! {
                        pub struct #autocomplete_ident #generics(
                            ::serenity_commands::Autocomplete<#ty>
                        );
                    },
                    quote! {
                        <
                            ::serenity_commands::Autocomplete<#ty> as ::serenity_commands::AutocompleteSubCommandOrGroup
                        >::from_value(value)
                            .map(self)
                    },
                )
            }
            Style::Unit => return None,
        };

        let ident = &self.ident;

        Some(quote! {
            #ty

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SupportsAutocomplete for #ident #ty_generics #where_clause {
                type Autocomplete = #autocomplete_ident #ty_generics;
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::AutocompleteSubCommandOrGroup for #autocomplete_ident #ty_generics #where_clause {
                fn from_value(
                    value: &::serenity::all::CommandDataOptionValue
                ) -> ::serenity_commands::Result<Self> {
                    #body
                }
            }
        })
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut acc = Error::accumulator();

        let ident = &self.ident;

        let create_option = self.create_option(&mut acc);
        let from_value = self.from_value();
        let autocomplete = self.autocomplete();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SubCommandGroup for #ident #ty_generics #where_clause {
                #create_option

                #from_value
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SubCommand for #ident #ty_generics #where_clause {}

            #autocomplete
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
