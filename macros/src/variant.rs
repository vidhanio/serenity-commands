use darling::{
    ast::{Fields, Style},
    error::Accumulator,
    util::{Flag, SpannedValue},
    Error, FromMeta, FromVariant,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, LitStr};

use crate::{
    field::Field,
    utils::{documentation_string, kebab_name, title_name},
    BuilderMethodList,
};

#[derive(Debug, PartialEq, FromMeta)]
enum ContextMenuType {
    User,
    Message,
}

impl ContextMenuType {
    fn command_type(&self) -> TokenStream {
        match self {
            Self::User => quote!(::serenity::all::CommandType::User),
            Self::Message => quote!(::serenity:all::CommandType::Message),
        }
    }
}

#[derive(Debug, FromVariant)]
#[darling(attributes(command), forward_attrs(doc))]
pub struct Variant {
    ident: Ident,
    fields: Fields<Field>,
    attrs: Vec<Attribute>,

    name: Option<SpannedValue<String>>,
    builder: Option<BuilderMethodList>,
    pub autocomplete: Flag,
    context_menu: Option<SpannedValue<ContextMenuType>>,
}

impl Variant {
    pub fn name_kebab(&self) -> LitStr {
        kebab_name(&self.ident, self.name.as_ref())
    }

    pub fn name_title(&self) -> LitStr {
        title_name(&self.ident, self.name.as_ref())
    }

    pub fn create_command(&self, acc: &mut Accumulator) -> TokenStream {
        let body = match self.fields.style {
            Style::Struct => {
                let name = self.name_kebab();
                let description = documentation_string(&self.attrs, &self.ident, acc);

                let fields = self.fields.iter().map(|field| field.create_option(acc));

                quote! {
                    ::serenity::all::CreateCommand::new(#name)
                        .description(#description)
                        .set_options(::std::vec![#(#fields),*])
                }
            }
            Style::Tuple => {
                let name = self.name_kebab();
                let description = documentation_string(&self.attrs, &self.ident, acc);

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
            Style::Unit => self.context_menu.as_ref().map_or_else(
                || {
                    let name = self.name_kebab();
                    let description = documentation_string(&self.attrs, &self.ident, acc);

                    quote! {
                        ::serenity::all::CreateCommand::new(#name)
                            .description(#description)
                    }
                },
                |ctx_menu| {
                    let name = self.name_title();
                    let ctx_type = ctx_menu.command_type();

                    quote! {
                        ::serenity::all::CreateCommand::new(#name)
                            .kind(#ctx_type)
                    }
                },
            ),
        };

        let builder_methods = &self.builder;

        quote! {
            #body
            #builder_methods
        }
    }

    pub fn create_sub_command_group(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name_kebab();
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

    pub fn create_sub_command(&self, acc: &mut Accumulator) -> TokenStream {
        let name = self.name_kebab();
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
                    {
                        const _: fn() = || {
                            fn assert_subcommand<T: ::serenity_commands::SubCommand>() {}
                            assert_subcommand::<#ty>();
                        };

                        <#ty as ::serenity_commands::SubCommandGroup>::create_option(
                            #name,
                            #description,
                        )
                    }
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
    pub fn from_command_options(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_inits) = Field::from_options(&self.fields.fields, false);

                quote! {
                    #fold

                    ::std::result::Result::Ok(Self::#ident {
                        #field_inits
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

        let name = self.name_kebab();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_subcommand_group_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_inits) = Field::from_options(&self.fields.fields, false);

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
                        #field_inits
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

        let name = self.name_kebab();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_subcommand_value(&self) -> TokenStream {
        let ident = &self.ident;

        let match_body = match self.fields.style {
            Style::Struct => {
                let (fold, field_inits) = Field::from_options(&self.fields.fields, false);

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
                        #field_inits
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
                    {
                        const _: fn() = || {
                            fn assert_subcommand<T: ::serenity_commands::SubCommand>() {}
                            assert_subcommand::<#ty>();
                        };

                        <#ty as ::serenity_commands::SubCommandGroup>::from_value(&option.value)
                            .map(Self::#ident)
                    }
                }
            }
            Style::Unit => {
                quote! {
                    ::std::result::Result::Ok(Self::#ident)
                }
            }
        };

        let name = self.name_kebab();

        quote! {
            #name => { #match_body }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_autocomplete_command_options(
        &self,
        acc: &mut Accumulator,
    ) -> Option<(TokenStream, TokenStream)> {
        if !self.autocomplete.is_present() {
            return None;
        }

        let ident = &self.ident;

        match self.fields.style {
            Style::Struct => {
                acc.push(
                    Error::custom("#[command(autocomplete)] is not supported on struct variants")
                        .with_span(&self.ident),
                );

                None
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;
                let name = self.name_kebab();

                Some((
                    quote!(
                        #ident(::serenity_commands::Autocomplete<#ty>)
                    ),
                    quote! {
                        #name => {
                            <
                                ::serenity_commands::Autocomplete<#ty>
                                    as ::serenity_commands::AutocompleteCommand
                            >::from_options(options).map(Self::#ident)
                        }
                    },
                ))
            }
            Style::Unit => {
                acc.push(
                    Error::custom("#[command(autocomplete)] is not supported on unit variants")
                        .with_span(&self.ident),
                );

                None
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_autocomplete_subcommand_or_group_value(
        &self,
        acc: &mut Accumulator,
    ) -> Option<(TokenStream, TokenStream)> {
        if !self.autocomplete.is_present() {
            return None;
        }

        let ident = &self.ident;

        match self.fields.style {
            Style::Struct => {
                acc.push(
                    Error::custom("#[command(autocomplete)] is not supported on struct variants")
                        .with_span(&self.ident),
                );

                None
            }
            Style::Tuple => {
                let field = self
                    .fields
                    .fields
                    .first()
                    .expect("`Args` should only accept tuple `enum` variants with one field");
                let ty = &field.ty;
                let name = self.name_kebab();

                Some((
                    quote!(
                        #ident(::serenity_commands::Autocomplete<#ty>)
                    ),
                    quote! {
                        #name => {
                            <
                                ::serenity_commands::Autocomplete<#ty>
                                    as ::serenity_commands::AutocompleteSubCommandOrGroup
                            >::from_value(&option.value).map(Self::#ident)
                        }
                    },
                ))
            }
            Style::Unit => {
                acc.push(
                    Error::custom("#[command(autocomplete)] is not supported on unit variants")
                        .with_span(&self.ident),
                );

                None
            }
        }
    }
}
