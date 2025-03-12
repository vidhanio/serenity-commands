use darling::{ast::Data, error::Accumulator, util::Ignored, Error, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, Ident};

use crate::{utils::IdentExt, variant::Variant};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(command), supports(enum_named, enum_newtype, enum_unit))]
pub struct Args {
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Ignored>,
}

impl Args {
    fn create_commands(&self, acc: &mut Accumulator) -> TokenStream {
        let commands = self
            .data
            .as_ref()
            .take_enum()
            .expect("`Args` should only accept `enum`s")
            .into_iter()
            .map(|variant| variant.create_command(acc));

        quote! {
            fn create_commands() -> ::std::vec::Vec<::serenity::all::CreateCommand> {
                ::std::vec![#(#commands),*]
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_command_data(&self) -> TokenStream {
        let variants = self
            .data
            .as_ref()
            .take_enum()
            .expect("`Args` should only accept `enum`s");

        let arms = variants.into_iter().map(Variant::from_command_options);

        quote! {
            fn from_command_data(
                data: &::serenity::all::CommandData
            ) -> ::serenity_commands::Result<Self> {
                if data.autocomplete().is_some() {
                    return ::std::result::Result::Err(
                        ::serenity_commands::Error::UnexpectedAutocompleteOption
                    );
                }
                let options = &data.options;

                match data.name.as_str() {
                    #(#arms,)*
                    _ => ::std::result::Result::Err(
                        ::serenity_commands::Error::UnknownCommand(
                            ::std::clone::Clone::clone(&data.name)
                        )
                    ),
                }
            }
        }
    }

    fn autocomplete(&self, acc: &mut Accumulator) -> Option<TokenStream> {
        let variants = self
            .data
            .as_ref()
            .take_enum()
            .expect("`Args` should only accept `enum`s");

        let (variants, arms) = variants
            .into_iter()
            .filter_map(|variant| variant.from_autocomplete_command_options(acc))
            .unzip::<_, _, Vec<_>, Vec<_>>();

        if variants.is_empty() {
            return None;
        }

        let ident = &self.ident;
        let autocomplete_ident = self.ident.autocomplete();

        let generics = &self.generics;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        Some(quote! {
            pub enum #autocomplete_ident #generics {
                #(#variants,)*
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SupportsAutocomplete for #ident #ty_generics #where_clause {
                type Autocomplete = #autocomplete_ident;
            }

            #[automatically_derived]
            impl #impl_generics ::serenity_commands::AutocompleteCommands for #autocomplete_ident #ty_generics #where_clause {
                fn from_command_data(
                    data: &::serenity::all::CommandData
                ) -> ::serenity_commands::Result<Self> {
                    if data.autocomplete().is_none() {
                        return ::std::result::Result::Err(
                            ::serenity_commands::Error::MissingAutocompleteOption
                        );
                    }

                    let options = &data.options;

                    match data.name.as_str() {
                        #(#arms,)*
                        _ => ::std::result::Result::Err(
                            ::serenity_commands::Error::UnknownCommand(
                                ::std::clone::Clone::clone(&data.name)
                            )
                        ),
                    }
                }
            }
        })
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut acc = Error::accumulator();

        let ident = &self.ident;

        let create_commands = self.create_commands(&mut acc);
        let from_command_data = self.from_command_data();
        let autocomplete = self.autocomplete(&mut acc);

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::Commands for #ident #ty_generics #where_clause {
                #create_commands

                #from_command_data
            }

            #autocomplete
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
