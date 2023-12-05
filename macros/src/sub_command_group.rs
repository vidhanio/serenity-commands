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
    fn create_option(&self, acc: &mut Accumulator) -> TokenStream {
        let variants = self.data.as_ref().take_enum().unwrap();

        let body = variants
            .iter()
            .map(|variant| variant.create_sub_command(acc));

        quote! {
            fn create_option(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommandOption {
                ::serenity::all::CreateCommandOption::new(
                    ::serenity::all::CommandOptionType::SubCommandGroup,
                    name,
                    description,
                )
                    #(.add_sub_option(#body))*
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_value(&self) -> TokenStream {
        let Data::Enum(variants) = &self.data else {
            unreachable!()
        };

        let arms = variants.iter().map(Variant::from_subcommand_value);

        quote! {
            fn from_value(
                value: &::serenity::all::CommandDataOptionValue,
            ) -> ::serenity_commands::Result<Self> {
                let ::serenity::all::CommandDataOptionValue::SubCommandGroup(options) = value else {
                    return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                        got: value.kind(),
                        expected: ::serenity::all::CommandOptionType::SubCommandGroup,
                    });
                };

                let [option] = options.as_slice() else {
                    return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionCount {
                        got: options.len(),
                        expected: 1,
                    });
                };

                match option.name.as_str() {
                    #(#arms)*
                    unknown => ::std::result::Result::Err(
                        ::serenity_commands::Error::UnknownCommandOption(
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

        let create_option = self.create_option(&mut acc);
        let from_value = self.from_value();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::SubCommandGroup for #ident #ty_generics #where_clause {
                #create_option

                #from_value
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}
