use std::iter;

use darling::{
    error::Accumulator,
    util::{Flag, SpannedValue},
    FromField,
};
use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, Index, LitStr, Path, Type};

use crate::{
    utils::{documentation_string, kebab_name},
    BuilderMethodList,
};

#[derive(Debug, FromField)]
#[darling(attributes(command), forward_attrs(doc))]
pub struct Field {
    ident: Option<Ident>,
    pub ty: Type,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,
    builder: Option<BuilderMethodList>,
    pub autocomplete: Flag,
    with: Option<Path>,
}

impl Field {
    pub const fn ident(&self) -> &Ident {
        self.ident
            .as_ref()
            .expect("`Field::ident` should only be called on named fields")
    }

    pub fn name(&self) -> LitStr {
        kebab_name(self.ident(), self.name.as_ref())
    }

    pub fn create_option(&self, acc: &mut Accumulator) -> TokenStream {
        let path_base = self.with.as_ref().map_or_else(
            || {
                let ty = &self.ty;

                quote!(<#ty as ::serenity_commands::BasicOption>)
            },
            |with| quote!(#with),
        );

        let ident = self.ident();

        let name = self.name();
        let description = documentation_string(&self.attrs, ident, acc);
        let builder_methods = &self.builder;
        let autocomplete = self.autocomplete.is_present().then(|| {
            quote! {
                .set_autocomplete(true)
            }
        });

        quote! {
            #path_base::create_option(
                #name,
                #description,
            )
            #autocomplete
            #builder_methods
        }
    }

    pub fn from_options(selfs: &[Self], autocomplete: bool) -> (TokenStream, TokenStream) {
        let match_arms = selfs.iter().enumerate().map(|(idx, field)| {
            let idx = Index::from(idx);
            let name = field.name();

            quote! {
                #name => acc.#idx = ::std::option::Option::Some(
                    &option.value
                )
            }
        });

        let tuple_inits = iter::repeat_n(quote!(::std::option::Option::None), selfs.len());

        let field_init = selfs.iter().enumerate().map(|(idx, field)| {
            let path_base = field.with.as_ref().map_or_else(
                || quote!(::serenity_commands::BasicOption),
                |with| quote!(#with),
            );

            let ident = field.ident();
            let idx = Index::from(idx);
            
            let constructor = if field.autocomplete.is_present() && autocomplete {
                quote!{
                    match acc.#idx {
                        ::std::option::Option::Some(::serenity::all::CommandDataOptionValue::Autocomplete {
                            value, ..
                        }) => {
                            let val = ::serenity::all::CommandDataOptionValue::String(
                                ::std::clone::Clone::clone(value)
                            );
                            ::serenity_commands::BasicOption::from_value(::std::option::Option::Some(&val))
                        }
                        _ => ::serenity_commands::BasicOption::from_value(acc.#idx)
                    }?
                }
            } else {
                quote! {
                    #path_base::from_value(acc.#idx)?
                }
            };

            quote! {
                #ident: #constructor
            }
        });

        let fold = quote! {
            let acc = ::std::iter::Iterator::try_fold(
                &mut options.iter(),
                (#(#tuple_inits,)*),
                |mut acc, option| {
                    match option.name.as_str() {
                        #(#match_arms,)*
                        _ => {
                            return ::std::result::Result::Err(
                                ::serenity_commands::Error::UnknownCommandOption(
                                    ::std::clone::Clone::clone(&option.name)
                                )
                            )
                        }
                    }

                    ::std::result::Result::Ok(acc)
                }
            )?;
        };

        (fold, quote!(#(#field_init,)*))
    }

    pub fn autocomplete_variant_ident(&self) -> Ident {
        let ident = self.ident();
        let ident_s = ident.to_string();
        Ident::new(&ident_s.strip_prefix("r#").unwrap_or(ident_s.as_str()).to_pascal_case(), ident.span())
    }

    pub fn autocomplete_variants(selfs: &[Self]) -> TokenStream {
        let variants = selfs
            .iter()
            .enumerate()
            .filter(|(_, field)| field.autocomplete.is_present())
            .map(|(i, field)| {
                let ident = field.autocomplete_variant_ident();

                let field_idents = selfs.iter().map(Self::ident);
                let field_types = selfs.iter().enumerate().map(|(j, field)| {
                    if i == j {
                        quote!(::std::string::String)
                    } else {
                        let ty = &field.ty;
                        quote!{
                            ::serenity_commands::PartialOption<#ty>
                        }
                    }
                });

                quote! {
                    #ident {
                        #[allow(dead_code)]
                        #(#field_idents: #field_types,)*
                    }
                }});

        quote!(#(#variants,)*)
    }

    pub fn from_autocomplete_options(selfs: &[Self]) -> TokenStream {
        let (fold, field_inits) = Self::from_options(selfs, true);

        let arms = selfs
            .iter()
            .filter(|field| field.autocomplete.is_present())
            .map(|field| {
                let ident = field.autocomplete_variant_ident();
                let name = field.name();

                quote! {
                    #name => ::std::result::Result::Ok(
                        Self::#ident {
                            #field_inits
                        }
                    )
                }
            });

        quote! {
            #fold

            ::std::iter::Iterator::find_map(
                &mut options.iter(),
                |option| {
                    if let ::serenity::all::CommandDataOptionValue::Autocomplete {
                        ..
                    } = &option.value {
                        Some((|| match option.name.as_str() {
                            #(#arms,)*
                            _ => ::std::result::Result::Err(
                                ::serenity_commands::Error::UnknownAutocompleteOption(
                                    ::std::clone::Clone::clone(&option.name)
                                )
                            )
                        })())
                    } else {
                        ::std::option::Option::None
                    }
                }
            )
            .unwrap_or(::std::result::Result::Err(
                ::serenity_commands::Error::MissingAutocompleteOption
            ))
        }
    }
}
