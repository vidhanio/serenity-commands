use darling::{ast::Data, util::SpannedValue, FromDeriveInput, FromMeta, FromVariant};
use heck::{ToKebabCase, ToTitleCase};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Generics, Ident, Lit, LitStr, Type};

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
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(choice), supports(enum_unit))]
pub struct Args {
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Type>,

    option_type: SpannedValue<OptionType>,
}

impl Args {
    fn create_option(&self) -> TokenStream {
        let choices = self
            .data
            .as_ref()
            .take_enum()
            .unwrap()
            .into_iter()
            .map(Variant::create_option_choice);

        let command_option_type = self.option_type.command_option_type();
        let method_name = self.option_type.method_name(self.option_type.span());

        quote! {
            fn create_option(
                name: impl ::std::convert::Into<::std::string::String>,
                description: impl ::std::convert::Into<::std::string::String>,
            ) -> ::serenity::all::CreateCommandOption {
                ::serenity::all::CreateCommandOption::new(
                    ::serenity::all::CommandOptionType::#command_option_type,
                    name,
                    description,
                )
                #(.#method_name(#choices))*
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_value(&self) -> TokenStream {
        let arms = self
            .data
            .as_ref()
            .take_enum()
            .unwrap()
            .into_iter()
            .map(Variant::from_value);

        let option_type = self.option_type.command_option_type();

        let choice_expr = if *self.option_type == OptionType::String {
            quote!(choice.as_str())
        } else {
            quote!(choice)
        };

        quote! {
            fn from_value(
                value: ::std::option::Option<&::serenity::all::CommandDataOptionValue>
            ) -> ::serenity_commands::Result<Self> {
                let value = value
                    .ok_or(::serenity_commands::Error::MissingRequiredCommandOption)?;

                let ::serenity::all::CommandDataOptionValue::#option_type(choice) = value else {
                    return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                        expected: ::serenity::all::CommandOptionType::#option_type,
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
    }
}

impl ToTokens for Args {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let create_option = self.create_option();
        let from_value = self.from_value();

        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_generics ::serenity_commands::BasicOption for #ident #ty_generics #where_clause {
                #create_option

                #from_value
            }
        }
        .to_tokens(tokens);
    }
}

#[derive(Debug, FromVariant)]
#[darling(attributes(choice))]
pub struct Variant {
    ident: Ident,
    name: Option<SpannedValue<String>>,

    value: Option<Lit>,
}

impl Variant {
    fn name(&self) -> LitStr {
        self.name.as_ref().map_or_else(
            || {
                let ident_s = self.ident.to_string();
                LitStr::new(
                    &ident_s
                        .strip_prefix("r#")
                        .unwrap_or(&ident_s)
                        .to_title_case(),
                    self.ident.span(),
                )
            },
            |name| LitStr::new(name, name.span()),
        )
    }

    fn value(&self) -> Lit {
        self.value.clone().unwrap_or_else(|| {
            let ident_s = self.ident.to_string();
            Lit::Str(LitStr::new(
                &ident_s
                    .strip_prefix("r#")
                    .unwrap_or(&ident_s)
                    .to_kebab_case(),
                self.ident.span(),
            ))
        })
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
