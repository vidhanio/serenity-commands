use darling::{ast::Data, error::Accumulator, util::Ignored, Error, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Generics, Ident};

use crate::Variant;

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
                let options = &data.options;

                match data.name.as_str() {
                    #(#arms,)*
                    unknown => ::std::result::Result::Err(
                        ::serenity_commands::Error::UnknownCommand(
                            ::std::borrow::ToOwned::to_owned(unknown)
                        )
                    ),
                }
            }
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut acc = Error::accumulator();

        let ident = &self.ident;

        let create_commands = self.create_commands(&mut acc);
        let from_command_data = self.from_command_data();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::Commands for #ident #ty_generics #where_clause {
                #create_commands

                #from_command_data
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
