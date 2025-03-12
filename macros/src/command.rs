use darling::{
    ast::{Data, Style},
    error::Accumulator,
    Error, FromDeriveInput,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, Ident};

use crate::{field::Field, utils::IdentExt, variant::Variant, BuilderMethodList};

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
    generics: Generics,
    data: Data<Variant, Field>,

    builder: Option<BuilderMethodList>,
}

impl Args {
    fn create_command(&self, acc: &mut Accumulator) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => match fields.style {
                Style::Struct => {
                    let options = fields.fields.iter().map(|field| field.create_option(acc));

                    quote! {
                        ::serenity::all::CreateCommand::new(name)
                            .description(description)
                            .set_options(::std::vec![#(#options),*])
                    }
                }
                Style::Tuple => {
                    let field = fields
                        .fields
                        .first()
                        .expect("`Args` should only accept tuple `struct`s with one field");
                    let ty = &field.ty;

                    quote! {
                        <#ty as ::serenity_commands::Command>::create_command(name, description)
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
                let options = variants
                    .iter()
                    .map(|variant| variant.create_sub_command_group(acc));

                quote! {
                    ::serenity::all::CreateCommand::new(name)
                        .description(description)
                        .set_options(::std::vec![#(#options),*])
                }
            }
        };

        let builder_methods = &self.builder;

        quote! {
            fn create_command(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommand {
                #body
                #builder_methods
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_options(&self) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => match fields.style {
                Style::Struct => {
                    let (fold, field_inits) = Field::from_options(&fields.fields);

                    quote! {
                        #fold

                        ::std::result::Result::Ok(Self {
                            #field_inits
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
                        <#ty as ::serenity_commands::Command>::from_command(options)
                            .map(Self)
                    }
                }
                Style::Unit => {
                    quote! {
                        ::std::result::Result::Ok(Self)
                    }
                }
            },
            Data::Enum(variants) => {
                let arms = variants.iter().map(Variant::from_subcommand_group_value);

                quote! {
                    let [option] = options else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionCount {
                            got: options.len(),
                            expected: 1,
                        });
                    };

                    match option.name.as_str() {
                        #(#arms,)*
                        _ => ::std::result::Result::Err(
                            ::serenity_commands::Error::UnknownCommandOption(
                                ::std::clone::Clone::clone(&option.name)
                            )
                        ),
                    }
                }
            }
        };

        quote! {
            fn from_options(
                options: &[::serenity::all::CommandDataOption],
            ) -> ::serenity_commands::Result<Self> {
                #body
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn autocomplete(&self, acc: &mut Accumulator) -> Option<TokenStream> {
        let autocomplete_ident = self.ident.autocomplete();
        let generics = &self.generics;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let (ty, body) = match &self.data {
            Data::Struct(fields) => match fields.style {
                Style::Struct => {
                    let variants = Field::autocomplete_variants(&fields.fields);

                    if variants.is_empty() {
                        return None;
                    }

                    (
                        quote! {
                            pub enum #autocomplete_ident #generics {
                                #variants
                            }
                        },
                        Field::from_autocomplete_options(&fields.fields),
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
                                ::serenity_commands::Autocomplete<#ty> as ::serenity_commands::AutocompleteCommand
                            >::from_command(options).map(Self)
                        },
                    )
                }
                Style::Unit => return None,
            },
            Data::Enum(variants) => {
                let (variants, arms) = variants
                    .iter()
                    .filter_map(|variant| variant.from_autocomplete_subcommand_or_group_value(acc))
                    .unzip::<_, _, Vec<_>, Vec<_>>();

                if variants.is_empty() {
                    return None;
                }

                (
                    quote! {
                        pub enum #autocomplete_ident #generics {
                            #(#variants,)*
                        }
                    },
                    quote! {
                        let [option] = options else {
                            return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionCount {
                                got: options.len(),
                                expected: 1,
                            });
                        };

                        match option.name.as_str() {
                            #(#arms,)*
                            _ => ::std::result::Result::Err(
                                ::serenity_commands::Error::UnknownCommandOption(
                                    ::std::clone::Clone::clone(&option.name)
                                )
                            ),
                        }
                    },
                )
            }
        };

        let ident = &self.ident;

        Some(quote! {
            #ty

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SupportsAutocomplete for #ident #ty_generics #where_clause {
                type Autocomplete = #autocomplete_ident #ty_generics;
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::AutocompleteCommand for #autocomplete_ident #ty_generics #where_clause {
                fn from_options(
                    options: &[::serenity::all::CommandDataOption],
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

        let create_command = self.create_command(&mut acc);
        let from_options = self.from_options();
        let autocomplete = self.autocomplete(&mut acc);

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::Command for #ident #ty_generics #where_clause {
                #create_command

                #from_options
            }

            #autocomplete
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
