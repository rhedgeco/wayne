use std::{env, fs::File, io::BufReader, path::PathBuf};

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

use super::xml::{Arg, ArgType, Enum, Event, Interface, Protocol, Request};

pub struct Generator {
    protocol_path: Path,
    protocol: Protocol,
}

impl Parse for Generator {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let protocol_path = input.parse::<Path>()?;
        let _ = input.parse::<Token![,]>()?;
        let file_path = input.parse::<LitStr>()?;
        let root_path: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
        match File::open(root_path.join(file_path.value())) {
            Err(err) => Err(syn::Error::new(file_path.span(), err)),
            Ok(file) => match quick_xml::de::from_reader::<_, Protocol>(BufReader::new(file)) {
                Err(err) => Err(syn::Error::new(file_path.span(), err)),
                Ok(protocol) => Ok(Self {
                    protocol_path,
                    protocol,
                }),
            },
        }
    }
}

impl ToTokens for Generator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let protocol_path = &self.protocol_path;
        let ident = utils::ident(&self.protocol.name);
        let interface_types = self.protocol.interfaces.iter().map(Type);
        tokens.extend(quote! {
            pub mod #ident {
                #[allow(unused_imports)]
                use ::std::os::fd::OwnedFd;

                // re-export useful protocol items
                #[allow(unused_imports)]
                use #protocol_path::{
                    Parser, Buffer,
                    parser::Builder,
                    types::{
                        RawEnum, RawString,
                        id::{NewId, CustomNewId, ObjectId},
                    },
                    parse::{
                        int, uint, float, string, custom, array, fd,
                    },
                };

                #(#interface_types)*
            }
        });
    }
}

struct Type<T>(T);
struct Parser<T>(T);

impl ToTokens for Type<&Interface> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let version = self.0.version;
        let mod_ident = utils::ident(&self.0.name);
        let request_types = self.0.requests.iter().map(Type);
        let event_types = self.0.events.iter().map(Type);
        let enum_types = self.0.enums.iter().map(Type);
        let docs = self.0.description.lines();

        let pascal_name = self.0.name.to_case(Case::Pascal);
        let main_enum = utils::ident(&pascal_name);

        let request_enum = utils::ident(&format!("{pascal_name}Request"));
        let request_variants = self.0.requests.iter().map(|request| {
            let name = request.name.to_case(Case::Pascal);
            let variant = utils::ident(&name);
            let item = utils::ident(format!("{name}Request"));
            quote! { #variant(#item) }
        });

        let event_enum = utils::ident(format!("{pascal_name}Event"));
        let event_variants = self.0.events.iter().map(|event| {
            let name = event.name.to_case(Case::Pascal);
            let variant = utils::ident(&name);
            let item = utils::ident(format!("{name}Event"));
            quote! { #variant(#item) }
        });

        let opcodes = 0..(self.0.requests.len() as u16);
        let parser_enum = utils::ident(&format!("{pascal_name}Parser"));
        let parser_variants = self.0.requests.iter().map(|request| {
            let name = request.name.to_case(Case::Pascal);
            let variant = utils::ident(&name);
            let item = utils::ident(format!("{name}Parser"));
            quote! { #variant(#item) }
        });
        let parser_init = self.0.requests.iter().map(|request| {
            let name = request.name.to_case(Case::Pascal);
            let variant = utils::ident(&name);
            let item = utils::ident(format!("{name}Parser"));
            quote! { Self::#variant(#item::new()) }
        });
        let parser_parse = self.0.requests.iter().map(|request| {
            let variant = utils::ident(request.name.to_case(Case::Pascal));
            quote! {
                Self::#variant(parser) => Some(
                    #request_enum::#variant(parser.parse(bytes, fds)?)
                )
            }
        });

        let request_parsers = self.0.requests.iter().map(Parser);

        tokens.extend(quote! {
            pub use #mod_ident::#main_enum;

            #(#[doc = #docs])*
            pub mod #mod_ident {
                use super::*;

                const VERSION: u32 = #version;

                #[derive(Debug)]
                pub enum #main_enum {
                    Request(#request_enum),
                    Event(#event_enum),
                }

                #[derive(Debug)]
                pub enum #request_enum {
                    #(#request_variants,)*
                }

                impl #request_enum {
                    pub fn parser(opcode: u16) -> Option<#parser_enum> {
                        #parser_enum::new(opcode)
                    }
                }

                #(#request_types)*

                #[derive(Debug)]
                pub enum #event_enum {
                    #(#event_variants,)*
                }

                #(#event_types)*

                #(#enum_types)*

                pub enum #parser_enum {
                    #(#parser_variants,)*
                }

                impl #parser_enum {
                    pub fn new(opcode: u16) -> Option<Self> {
                        match opcode {
                            #(#opcodes => Some(#parser_init),)*
                            _ => None,
                        }
                    }
                }

                impl Parser for #parser_enum {
                    type Output = #request_enum;

                    fn parse(&mut self, bytes: impl Buffer<u8>, fds: impl Buffer<OwnedFd>) -> Option<Self::Output> {
                        match self {
                            #(#parser_parse,)*
                            _ => unreachable!(),
                        }
                    }
                }

                #(#request_parsers)*
            }
        });
    }
}

impl ToTokens for Type<&Request> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = self.0.description.lines();
        let name = self.0.name.to_case(Case::Pascal);
        let ident = utils::ident(format!("{name}Request"));
        let args = self.0.args.iter().map(Type);

        tokens.extend(quote! {
            #(#[doc = #docs])*
            #[derive(Debug)]
            pub struct #ident {
                #(#args)*
            }
        });
    }
}

impl ToTokens for Parser<&Request> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = self.0.name.to_case(Case::Pascal);
        let ident = utils::ident(format!("{name}Request"));
        let parser = utils::ident(format!("{name}Parser"));
        let arg_ty = self
            .0
            .args
            .iter()
            .map(|arg| match &arg.ty {
                ArgType::Int => utils::ident("int"),
                ArgType::Uint => utils::ident("uint"),
                ArgType::Fixed => utils::ident("float"),
                ArgType::String => utils::ident("string"),
                ArgType::Object => utils::ident("uint"),
                ArgType::Array => utils::ident("array"),
                ArgType::Fd => utils::ident("fd"),
                ArgType::NewId => match &arg.interface {
                    Some(_) => utils::ident("uint"),
                    None => utils::ident("custom"),
                },
            })
            .collect::<Box<[_]>>();

        let arg_name = self
            .0
            .args
            .iter()
            .map(|arg| utils::ident(&arg.name))
            .collect::<Box<[_]>>();

        tokens.extend(quote! {
            pub struct #parser {
                #(#arg_name: Builder<#arg_ty::Parser>,)*
            }

            impl #parser {
                pub fn new() -> Self {
                    Self {
                        #(#arg_name: Builder::new(#arg_ty::Parser::new()),)*
                    }
                }
            }

            impl Parser for #parser {
                type Output = #ident;

                fn parse(
                    &mut self,
                    mut bytes: impl Buffer<u8>,
                    mut fds: impl Buffer<OwnedFd>
                ) -> Option<Self::Output> {
                    #(self.#arg_name.parse(&mut bytes, &mut fds)?;)*

                    Some(#ident {
                        #(#arg_name: self.#arg_name.finish()?.into(),)*
                    })
                }
            }
        });
    }
}

impl ToTokens for Type<&Event> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = utils::ident(format!("{}Event", self.0.name.to_case(Case::Pascal)));
        let type_args = self.0.args.iter().map(Type);
        let docs = self.0.description.lines();

        tokens.extend(quote! {
            #(#[doc = #docs])*
            #[derive(Debug)]
            pub struct #ident {
                #(#type_args)*
            }
        });
    }
}

impl ToTokens for Type<&Enum> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = utils::ident(self.0.name.to_case(Case::Pascal));
        let docs = self.0.description.lines();

        let entry_iter = self.0.entries.iter();
        let entry_docs = entry_iter.clone().map(|entry| &entry.summary);
        let entry_names = entry_iter
            .clone()
            .map(|entry| utils::ident(&entry.name.to_case(Case::Pascal)))
            .collect::<Box<[_]>>();
        let entry_values = entry_iter.map(|entry| entry.value).collect::<Box<[_]>>();

        tokens.extend(quote! {
            #(#[doc = #docs])*
            #[derive(Debug)]
            #[repr(u32)]
            pub enum #ident {
                #(
                    #[doc = #entry_docs]
                    #entry_names = #entry_values,
                )*
            }

            impl TryFrom<u32> for #ident {
                type Error = ();
                fn try_from(value: u32) -> Result<Self, Self::Error> {
                    Ok(match value {
                        #(#entry_values => Self::#entry_names,)*
                        _ => return Err(()),
                    })
                }
            }
        });
    }
}

impl ToTokens for Type<&Arg> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.0.summary;
        let ident = utils::ident(&self.0.name);

        let interface = self
            .0
            .interface
            .as_ref()
            .map(|i| utils::ident(i.to_case(Case::Pascal)));

        let mut arg_ty = match &self.0.ty {
            ArgType::Int => quote! { i32 },
            ArgType::Uint => quote! { u32 },
            ArgType::Fixed => quote! { f32 },
            ArgType::String => quote! { RawString },
            ArgType::Array => quote! { Box<[u8]> },
            ArgType::Fd => quote! { OwnedFd },
            ArgType::Object => match &interface {
                Some(interface) => quote! { ObjectId<#interface> },
                None => quote! { ObjectId<()> },
            },
            ArgType::NewId => match &interface {
                Some(interface) => quote! { NewId<#interface> },
                None => quote! { CustomNewId },
            },
        };

        if let Some(kind) = &self.0.enum_kind {
            let mut parts = kind.split(".");
            let first = parts.next().unwrap();
            let kind = match parts.next() {
                None => utils::ident(first.to_case(Case::Pascal)).into_token_stream(),
                Some(second) => {
                    let interface = utils::ident(first);
                    let ident = utils::ident(second.to_case(Case::Pascal));
                    quote! { #interface::#ident }
                }
            };

            arg_ty = quote! { RawEnum<#arg_ty, #kind> };
        }

        tokens.extend(quote! {
            #[doc = #docs]
            #ident: #arg_ty,
        });
    }
}

impl ToTokens for Parser<&ArgType> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ty = match &self.0 {
            ArgType::Int => utils::ident("int"),
            ArgType::Uint => utils::ident("uint"),
            ArgType::Fixed => utils::ident("float"),
            ArgType::String => utils::ident("string"),
            ArgType::Object => utils::ident("uint"),
            ArgType::NewId => utils::ident("uint"),
            ArgType::Array => utils::ident("array"),
            ArgType::Fd => utils::ident("fd"),
        };

        tokens.extend(quote! { #ty })
    }
}

mod utils {
    use proc_macro2::Span;
    use syn::Ident;

    pub fn ident(s: impl AsRef<str>) -> Ident {
        let s = s.as_ref();
        match s.starts_with(|c: char| c.is_numeric()) {
            true => Ident::new(&format!("_{s}"), Span::call_site()),
            false => Ident::new(s, Span::call_site()),
        }
    }
}
