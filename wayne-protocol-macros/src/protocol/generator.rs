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
        let interface_data = self.protocol.interfaces.iter().map(Data);
        tokens.extend(quote! {
            pub mod #ident {
                #[allow(unused_imports)]
                use ::std::os::fd::OwnedFd;

                // re-export useful protocol items
                #[allow(unused_imports)]
                use #protocol_path::{
                    parse::{self, MapExt, OptionExt, ParseResult, Parser, Pass, ThenExt},
                    types::{ObjId, NewId},
                };

                #(#interface_data)*
            }
        });
    }
}

struct Data<T>(T);

impl ToTokens for Data<&Interface> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let version = self.0.version;
        let mod_ident = utils::ident(&self.0.name);
        let request_data = self.0.requests.iter().map(Data);
        let event_data = self.0.events.iter().map(Data);
        let enum_data = self.0.enums.iter().map(Data);
        let docs = self.0.description.lines();

        let pascal_name = self.0.name.to_case(Case::Pascal);
        let main_enum = utils::ident(&pascal_name);

        let request_enum = utils::ident(&format!("{pascal_name}Request"));
        let request_iter = self.0.requests.iter();
        let request_variants = request_iter
            .clone()
            .map(|request| utils::ident(request.name.to_case(Case::Pascal)))
            .collect::<Box<[_]>>();
        let request_items = request_iter
            .clone()
            .map(|request| {
                let name = request.name.to_case(Case::Pascal);
                utils::ident(&format!("{name}Request"))
            })
            .collect::<Box<[_]>>();

        let parser = utils::ident(&format!("{pascal_name}Parser"));
        let opcodes = (0..(self.0.requests.len() as u16)).collect::<Box<[_]>>();
        let parser_generics = opcodes
            .iter()
            .map(|i| utils::ident(&format!("P{i}")))
            .collect::<Box<[_]>>();

        let event_enum = utils::ident(format!("{pascal_name}Event"));
        let event_iter = self.0.events.iter();
        let event_variants = event_iter
            .clone()
            .map(|event| utils::ident(event.name.to_case(Case::Pascal)))
            .collect::<Box<[_]>>();
        let event_items = event_iter
            .clone()
            .map(|event| {
                let name = event.name.to_case(Case::Pascal);
                utils::ident(&format!("{name}Event"))
            })
            .collect::<Box<[_]>>();

        tokens.extend(quote! {
            pub use #mod_ident::#main_enum;

            #(#[doc = #docs])*
            pub mod #mod_ident {
                use super::*;

                const VERSION: u32 = #version;

                pub enum #main_enum {
                    Request(#request_enum),
                    Event(#event_enum),
                }

                pub enum #request_enum {
                    #(#request_variants(#request_items),)*
                }

                impl #request_enum {
                    pub fn parser(opcode: u16) -> Option<#parser<#(
                        impl Parser<Output = #request_items>,
                    )*>> {
                        match opcode {
                            #(#opcodes => Some(#parser::#request_variants(#request_items::parser())),)*
                            _ => None,
                        }
                    }
                }

                pub enum #parser<#(#parser_generics,)*>
                where #(#parser_generics: Parser<Output = #request_items>,)*
                {
                    #(#request_variants(#parser_generics),)*
                }

                impl<#(#parser_generics,)*> Parser for #parser<#(#parser_generics,)*>
                where #(#parser_generics: Parser<Output = #request_items>,)*
                {
                    type Output = #request_enum;
                    fn parse(self, buffer: impl parse::Buffer) -> ParseResult<Self> {
                        match self {
                            #(
                                #parser::#request_variants(parser) => parser
                                    .parse(buffer)
                                    .map(|request| Self::Output::#request_variants(request))
                                    .map_err(|err| err.map(|parser| Self::#request_variants(parser))),
                            )*
                        }
                    }
                }

                pub enum #event_enum {
                    #(#event_variants(#event_items),)*
                }

                #(#request_data)*
                #(#event_data)*
                #(#enum_data)*
            }
        });
    }
}

impl ToTokens for Data<&Request> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = utils::ident(format!("{}Request", self.0.name.to_case(Case::Pascal)));
        let args = self.0.args.iter().map(Data);
        let docs = self.0.description.lines();

        let arg_parser = self.0.args.iter().rev().fold(
            {
                let names = self.0.args.iter().map(|arg| utils::ident(&arg.name));
                quote! {
                    Pass::new(Self {
                        #(#names,)*
                    })
                }
            },
            |child, arg| {
                let name = utils::ident(&arg.name);
                let ty = match &arg.ty {
                    ArgType::Int => utils::ident("i32"),
                    ArgType::Uint => utils::ident("u32"),
                    ArgType::Fixed => utils::ident("f32"),
                    ArgType::String => utils::ident("string"),
                    ArgType::Object => utils::ident("obj_id"),
                    ArgType::NewId => utils::ident("new_id"),
                    ArgType::Array => utils::ident("array"),
                    ArgType::Fd => utils::ident("fd"),
                };

                let mapper = arg.enum_kind.as_ref().map(|kind| {
                    let kind = utils::enum_kind(kind);
                    quote! {.map(|v| #kind::parse(v as u32)).some()}
                });

                quote! {
                    parse::#ty()#mapper.then(move |#name| {
                        #child
                    })
                }
            },
        );

        tokens.extend(quote! {
            #(#[doc = #docs])*
            pub struct #ident {
                #(#args,)*
            }

            impl #ident {
                pub fn parser() -> impl Parser<Output = Self> {
                    #arg_parser
                }
            }
        });
    }
}

impl ToTokens for Data<&Event> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = utils::ident(format!("{}Event", self.0.name.to_case(Case::Pascal)));
        let args = self.0.args.iter().map(Data);
        let docs = self.0.description.lines();

        tokens.extend(quote! {
            #(#[doc = #docs])*
            pub struct #ident {
                #(#args,)*
            }
        });
    }
}

impl ToTokens for Data<&Enum> {
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
            #[repr(u32)]
            pub enum #ident {
                #(
                    #[doc = #entry_docs]
                    #entry_names = #entry_values,
                )*
            }

            impl #ident {
                pub fn parse(value: u32) -> Option<Self> {
                    match value {
                        #(
                            #entry_values => Some(Self::#entry_names),
                        )*
                        _ => None,
                    }
                }
            }
        });
    }
}

impl ToTokens for Data<&Arg> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.0.summary;
        let ident = utils::ident(&self.0.name);
        let enum_kind = self.0.enum_kind.as_ref().map(utils::enum_kind);

        let interface = match &self.0.interface {
            Some(interface) => utils::ident(interface.to_case(Case::Pascal)).into_token_stream(),
            None => quote! { () },
        };

        let arg_ty = match self.0.ty {
            ArgType::Fixed => quote! { f32 },
            ArgType::String => quote! { String },
            ArgType::Array => quote! { Box<[u8]> },
            ArgType::Fd => quote! { OwnedFd },
            ArgType::Object => quote! { ObjId<#interface> },
            ArgType::NewId => quote! { NewId<#interface> },
            ArgType::Int => enum_kind.unwrap_or_else(|| quote! { i32 }),
            ArgType::Uint => enum_kind.unwrap_or_else(|| quote! { u32 }),
        };

        tokens.extend(quote! {
            #[doc = #docs]
            #ident: #arg_ty
        });
    }
}

mod utils {
    use convert_case::{Case, Casing};
    use proc_macro2::{Span, TokenStream};
    use quote::{ToTokens, quote};
    use syn::Ident;

    pub fn ident(s: impl AsRef<str>) -> Ident {
        let s = s.as_ref();
        match s.starts_with(|c: char| c.is_numeric()) {
            true => Ident::new(&format!("_{s}"), Span::call_site()),
            false => Ident::new(s, Span::call_site()),
        }
    }

    pub fn enum_kind(s: impl AsRef<str>) -> TokenStream {
        let mut parts = s.as_ref().split(".");
        let first = parts.next().unwrap();
        match parts.next() {
            None => ident(first.to_case(Case::Pascal)).into_token_stream(),
            Some(second) => {
                let interface = ident(first);
                let ident = ident(second.to_case(Case::Pascal));
                quote! { #interface::#ident }
            }
        }
    }
}
