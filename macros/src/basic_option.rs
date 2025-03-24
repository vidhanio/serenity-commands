use darling::{
    ast::Data, error::Accumulator, util::SpannedValue, Error, FromDeriveInput, FromMeta,
    FromVariant,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{Generics, Ident, Lit, LitStr, Type};

use crate::{
    utils::{title_name, IdentExt},
    BuilderMethodList,
};

#[derive(Debug, PartialEq, FromMeta)]
enum OptionType {
    String,
    Integer,
    Number,
}

impl OptionType {
    fn command_option_type(&self) -> TokenStream {
        match self {
            Self::String => quote!(String),
            Self::Integer => quote!(Integer),
            Self::Number => quote!(Number),
        }
    }

    fn method_name(&self, span: Span) -> Ident {
        match self {
            Self::String => Ident::new("add_string_choice", span),
            Self::Integer => Ident::new("add_int_choice", span),
            Self::Number => Ident::new("add_number_choice", span),
        }
    }

    fn partial_type(&self, span: Span) -> TokenStream {
        match self {
            Self::String => quote_spanned!(span=> ::std::string::String),
            Self::Integer => quote_spanned!(span=> ::std::primitive::i64),
            Self::Number => quote_spanned!(span=> ::std::primitive::f64),
        }
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(option), supports(enum_unit, struct_newtype))]
pub struct Args {
    ident: Ident,
    generics: Generics,
    data: Data<ChoiceVariant, Type>,

    option_type: Option<SpannedValue<OptionType>>,

    builder: Option<BuilderMethodList>,
}

impl Args {
    fn create_option(&self) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => {
                let ty = fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `struct`s with one field");

                quote! {
                    <#ty as ::serenity_commands::BasicOption>::create_option(name, description)
                }
            }
            Data::Enum(variants) => {
                let choices = variants.iter().map(ChoiceVariant::create_option_choice);

                let Some(option_type) = &self.option_type else {
                    return quote!();
                };

                let command_option_type = option_type.command_option_type();
                let method_name = option_type.method_name(option_type.span());
                let builder_methods = &self.builder;

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::#command_option_type,
                        name,
                        description,
                    )
                    #(.#method_name(#choices))*
                    .required(true)
                    #builder_methods
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
    fn from_value(&self, acc: &mut Accumulator) -> TokenStream {
        let body = match &self.data {
            Data::Struct(fields) => {
                let ty = fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `struct`s with one field");

                quote! {
                    <#ty as ::serenity_commands::BasicOption>::from_value(value)
                        .map(Self)
                }
            }
            Data::Enum(variants) => {
                let arms = variants.iter().map(ChoiceVariant::from_value);

                let Some(option_type) = &self.option_type else {
                    acc.push(Error::custom(
                        "missing #[option(option_type = ...)] attribute",
                    ));

                    return quote!();
                };

                let command_option_type = option_type.command_option_type();

                let choice_expr = if **option_type == OptionType::String {
                    quote!(choice.as_str())
                } else {
                    quote!(choice)
                };

                quote! {
                    let value = value
                        .ok_or(::serenity_commands::Error::MissingRequiredCommandOption)?;

                    let ::serenity::all::CommandDataOptionValue::#command_option_type(choice) = value else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            expected: ::serenity::all::CommandOptionType::#command_option_type,
                            got: value.kind(),
                        });
                    };

                    match #choice_expr {
                        #(#arms)*
                        unknown => ::std::result::Result::Err(
                            ::serenity_commands::Error::UnknownChoice(
                                ::std::string::ToString::to_string(unknown)
                            )
                        )
                    }
                }
            }
        };

        quote! {
            fn from_value(
                value: ::std::option::Option<&::serenity::all::CommandDataOptionValue>
            ) -> ::serenity_commands::Result<Self> {
                #body
            }
        }
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let mut acc = Error::accumulator();

        let partial_type = match &self.data {
            Data::Struct(fields) => {
                let ty = fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `struct`s with one field");

                quote! {
                    <#ty as ::serenity_commands::BasicOption>::Partial
                }
            }
            Data::Enum(_) => self.option_type.as_ref().map_or_else(
                || {
                    acc.push(Error::custom(
                        "missing #[option(option_type = ...)] attribute",
                    ));

                    quote!(::std::string::String)
                },
                |option_type| option_type.partial_type(option_type.span()),
            ),
        };

        let create_option = self.create_option();
        let from_value = self.from_value(&mut acc);

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        let implementation = quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::BasicOption for #ident #ty_generics #where_clause {
                type Partial = #partial_type;

                #create_option

                #from_value
            }
        };

        acc.finish_with(implementation)
            .unwrap_or_else(Error::write_errors)
            .to_tokens(tokens);
    }
}

#[derive(Debug, FromVariant)]
#[darling(attributes(option))]
struct ChoiceVariant {
    ident: Ident,

    name: Option<SpannedValue<String>>,
    value: Option<Lit>,
}

impl ChoiceVariant {
    fn name(&self) -> LitStr {
        title_name(&self.ident, self.name.as_ref())
    }

    fn value(&self) -> Lit {
        self.value
            .clone()
            .unwrap_or_else(|| self.ident.to_kebab_case().into())
    }

    fn create_option_choice(&self) -> TokenStream {
        let name = self.name();
        let value = self.value();

        quote!(#name, #value)
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_value(&self) -> TokenStream {
        let value = self.value();
        let ident = &self.ident;

        quote! {
            #value => ::std::result::Result::Ok(Self::#ident),
        }
    }
}
