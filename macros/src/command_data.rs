use darling::{ast::Data, error::Accumulator, util::Ignored, Error, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;

use crate::Variant;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(command), supports(enum_named, enum_newtype, enum_unit))]
pub struct Args {
    ident: Ident,
    data: Data<Variant, Ignored>,
}

impl Args {
    fn to(&self, acc: &mut Accumulator) -> TokenStream {
        let variants = self
            .data
            .as_ref()
            .take_enum()
            .expect("`Args` should only accept `enum`s");

        let variants = variants
            .iter()
            .map(|variant| variant.to_data(acc))
            .collect::<Vec<_>>();

        quote! {
            fn to_command_data() -> ::std::vec::Vec<::serenity::all::CreateCommand> {
                ::std::vec![#(#variants),*]
            }
        }
    }

    fn from(&self) -> TokenStream {
        let variants = self
            .data
            .as_ref()
            .take_enum()
            .expect("`Args` should only accept `enum`s");

        let arms = variants.into_iter().map(Variant::from_data);

        quote! {
            fn from_command_data(
                data: &::serenity::all::CommandData
            ) -> ::serenity_commands::Result<Self> {
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

        let to = self.to(&mut acc);
        let from = self.from();

        let ident = &self.ident;

        let implementation = quote! {
            #[automatically_derived]
            impl ::serenity_commands::CommandData for #ident {
                #to

                #from
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
