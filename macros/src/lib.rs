#![allow(missing_docs)]
//! Macros for the `serenity_commands` crate.
//!
//! An implementation detail. Do not use directly.

mod basic_option;
mod command;
mod commands;
mod sub_command;
mod sub_command_group;

use std::iter;

use darling::{
    ast::{Fields, NestedMeta, Style},
    error::Accumulator,
    util::SpannedValue,
    Error, FromDeriveInput, FromField, FromMeta, FromVariant,
};
use heck::ToKebabCase;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
    Attribute, Expr, ExprLit, Ident, Index, Lit, LitStr, MacroDelimiter, Meta, MetaNameValue,
    Token, Type,
};

#[derive(Debug, FromVariant)]
#[darling(attributes(command), forward_attrs(doc))]
struct Variant {
    ident: Ident,
    fields: Fields<Field>,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,
    builder: Option<BuilderMethodList>,
}

impl Variant {
    fn name(&self) -> LitStr {
        option_name(&self.ident, self.name.as_ref())
    }

    fn create_command(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        let body = match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

                quote! {
                    ::serenity::all::CreateCommand::new(#name)
                        .description(#description)
                        .set_options(::std::vec![#(#fields),*])
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::Command>::create_command(#name, #description)
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommand::new(#name)
                        .description(#description)
                }
            }
        };

        let builder_methods = &self.builder;

        quote! {
            #body
            #builder_methods
        }
    }

    fn create_sub_command_or_group(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        let body = match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                    #(.add_sub_option(#fields))*
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommandGroup>::create_option(
                        #name,
                        #description,
                    )
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                }
            }
        };

        let builder_methods = &self.builder;

        quote! {
            #body
            #builder_methods
        }
    }

    fn create_sub_command(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name();
        let description = documentation_string(&self.attrs, &self.ident, acc);

        let body = match self.fields.style {
            Style::Struct => {
                let fields = self.fields.iter().map(|field| field.create_option(acc));

                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                    #(.add_sub_option(#fields))*
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommand>::create_option(
                        #name,
                        #description,
                    )
                }
            }
            Style::Unit => {
                quote! {
                    ::serenity::all::CreateCommandOption::new(
                        ::serenity::all::CommandOptionType::SubCommand,
                        #name,
                        #description,
                    )
                }
            }
        };

        let builder_methods = &self.builder;

        quote! {
            #body
            #builder_methods
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_command_options(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

                quote! {
                    #fold

                    ::std::result::Result::Ok(Self::#ident {
                        #(#field_init),*
                    })
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::Command>::from_options(
                        options
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_subcommand_or_group_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

                quote! {
                    let ::serenity::all::CommandDataOption {
                        value: ::serenity::all::CommandDataOptionValue::SubCommand(options),
                        ..
                    } = option else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: option.kind(),
                            expected: ::serenity::all::CommandOptionType::SubCommand,
                        });
                    };

                    #fold

                    ::std::result::Result::Ok(Self::#ident {
                        #(#field_init),*
                    })
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommandGroup>::from_value(
                        &option.value
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_subcommand_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_init) = Field::from_options(&self.fields.fields);

                quote! {
                    let ::serenity::all::CommandDataOption {
                        value: ::serenity::all::CommandDataOptionValue::SubCommand(options),
                        ..
                    } = option else {
                        return ::std::result::Result::Err(::serenity_commands::Error::IncorrectCommandOptionType {
                            got: value.kind(),
                            expected: ::serenity::all::CommandOptionType::SubCommand,
                        });
                    };

                    #fold

                    ::std::result::Result::Ok(Self::#ident {
                        #(#field_init),*
                    })
                }
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;

                quote! {
                    <#ty as ::serenity_commands::SubCommand>::from_value(
                        &option.value
                    ).map(Self::#ident)
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name();

        quote! {
            #name => { #match_body }
        }
    }
}

#[derive(Debug)]
struct DetachedMethodCall {
    method: Ident,
    #[allow(dead_code)]
    paren_token: Paren,
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for DetachedMethodCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            method: input.parse()?,
            paren_token: syn::parenthesized!(content in input),
            args: content.parse_terminated(Expr::parse, Token![,])?,
        })
    }
}

impl FromMeta for DetachedMethodCall {
    fn from_meta(item: &Meta) -> darling::Result<Self> {
        let Meta::List(list) = item else {
            return Err(Error::unsupported_format("non-meta list"));
        };

        let method = list
            .path
            .get_ident()
            .cloned()
            .ok_or_else(|| Error::unsupported_format("non-ident path"))?;

        let MacroDelimiter::Paren(paren_token) = list.delimiter else {
            return Err(Error::unsupported_format("non-parenthesized arguments"));
        };

        let args = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(list.tokens.clone())?;

        Ok(Self {
            method,
            paren_token,
            args,
        })
    }
}

#[derive(Debug)]
struct BuilderMethodList {
    methods: Vec<DetachedMethodCall>,
}

impl FromMeta for BuilderMethodList {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let methods = items
            .iter()
            .map(DetachedMethodCall::from_nested_meta)
            .collect::<darling::Result<_>>()?;

        Ok(Self { methods })
    }
}

impl ToTokens for BuilderMethodList {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let methods = self.methods.iter().map(|method| {
            let method_name = &method.method;
            let args = &method.args;

            quote_spanned! {method_name.span()=>
                .#method_name(#args)
            }
        });

        tokens.extend(quote! {
            #(#methods)*
        });
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(command), forward_attrs(doc))]
struct Field {
    ident: Option<Ident>,
    ty: Type,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,

    builder: Option<BuilderMethodList>,
}

impl Field {
    fn ident(&self) -> &Ident {
        self.ident
            .as_ref()
            .expect("`Field::ident` should only be called on named fields")
    }

    fn name(&self) -> LitStr {
        option_name(self.ident(), self.name.as_ref())
    }

    fn create_option(&self, acc: &mut Accumulator) -> TokenStream {
        let ident = self.ident();
        let ty = &self.ty;

        let name = self.name();
        let description = documentation_string(&self.attrs, ident, acc);
        let builder_methods = &self.builder;

        quote! {
            <#ty as ::serenity_commands::BasicOption>::create_option(
                #name,
                #description,
            )
            #builder_methods
        }
    }

    fn from_options(selfs: &[Self]) -> (TokenStream, impl Iterator<Item = TokenStream> + '_) {
        let match_arms = selfs.iter().enumerate().map(|(idx, field)| {
            let idx = Index::from(idx);
            let name = field.name();

            quote! {
                #name => acc.#idx = ::std::option::Option::Some(
                    &option.value
                )
            }
        });

        let inits = iter::repeat(quote!(::std::option::Option::None)).take(selfs.len());

        let field_init = selfs.iter().enumerate().map(|(idx, field)| {
            let ident = field.ident();
            let ty = &field.ty;

            let idx = Index::from(idx);

            quote! {
                #ident: <#ty as ::serenity_commands::BasicOption>::from_value(
                    acc.#idx
                )?
            }
        });

        let fold = quote! {
            let acc = ::std::iter::Iterator::fold(
                options.iter(),
                (#(#inits,)*),
                |mut acc, option| {
                    match option.name.as_str() {
                        #(#match_arms,)*
                        _ => {}
                    }

                    acc
                }
            );
        };

        (fold, field_init)
    }
}

#[proc_macro_derive(Commands, attributes(command))]
pub fn derive_commands(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    commands::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    command::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(SubCommandGroup, attributes(command))]
pub fn derive_sub_command_group(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sub_command_group::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(SubCommand, attributes(command))]
pub fn derive_sub_command(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sub_command::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

#[proc_macro_derive(BasicOption, attributes(choice))]
pub fn derive_basic_option(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    basic_option::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}

fn documentation_string(
    attrs: &[Attribute],
    spanned: &impl Spanned,
    acc: &mut Accumulator,
) -> LitStr {
    let mut doc_comments = attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) if path.is_ident("doc") => Some((s.span(), s.value())),
            _ => None,
        })
        .peekable();

    let res = if doc_comments.peek().is_none() {
        Err(
            Error::custom("missing documentation comment (`///`) to use as description")
                .with_span(spanned),
        )
    } else {
        let (span, s) = doc_comments.fold(
            (Span::call_site(), String::new()),
            |(span, mut acc), (_, s)| {
                if !acc.is_empty() {
                    acc.push(' ');
                }

                acc.push_str(s.trim());

                (span, acc)
            },
        );

        Ok(LitStr::new(&s, span))
    };

    acc.handle(res)
        .unwrap_or_else(|| LitStr::new("", Span::call_site()))
}

fn option_name(ident: &Ident, s: Option<&SpannedValue<String>>) -> LitStr {
    s.map_or_else(
        || {
            let ident_s = ident.to_string();
            LitStr::new(
                &ident_s
                    .strip_prefix("r#")
                    .unwrap_or(&ident_s)
                    .to_kebab_case(),
                ident.span(),
            )
        },
        |name| LitStr::new(name, name.span()),
    )
}
