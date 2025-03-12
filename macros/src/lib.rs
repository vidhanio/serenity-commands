#![allow(missing_docs)]
//! Macros for the `serenity_commands` crate.
//!
//! An implementation detail. Do not use directly.

mod basic_option;
mod command;
mod commands;
mod field;
mod sub_command;
mod sub_command_group;
mod utils;
mod variant;

use darling::{ast::NestedMeta, Error, FromDeriveInput, FromMeta};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    token::Paren,
    Expr, Ident, MacroDelimiter, Meta, Token,
};

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

#[proc_macro_derive(BasicOption, attributes(option))]
pub fn derive_basic_option(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    basic_option::Args::from_derive_input(&parse_macro_input!(tokens))
        .map_or_else(Error::write_errors, ToTokens::into_token_stream)
        .into()
}
