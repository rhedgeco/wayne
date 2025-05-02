use std::{env, fs::File, io::BufReader, path::PathBuf};

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

use super::xml::{Arg, ArgType, Entry, Enum, Event, Interface, Protocol, Request};

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

                // re-export all protocol items
                mod __protocol {
                    pub use #protocol_path::*;
                }

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
        let request_variants = self.0.requests.iter().map(|request| {
            let name = request.name.to_case(Case::Pascal);
            let item = utils::ident(format!("{name}Request"));
            let variant = utils::ident(name);
            quote! { #variant(#item) }
        });

        let event_enum = utils::ident(format!("{pascal_name}Event"));
        let event_variants = self.0.events.iter().map(|event| {
            let name = event.name.to_case(Case::Pascal);
            let item = utils::ident(format!("{name}Event"));
            let variant = utils::ident(name);
            quote! { #variant(#item) }
        });

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
                    #(#request_variants,)*
                }

                pub enum #event_enum {
                    #(#event_variants,)*
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

        tokens.extend(quote! {
            #(#[doc = #docs])*
            pub struct #ident {
                #(#args,)*
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
        let entries = self.0.entries.iter().map(Data);
        let docs = self.0.description.lines();

        tokens.extend(quote! {
            #(#[doc = #docs])*
            #[repr(u32)]
            pub enum #ident {
                #(#entries,)*
            }
        });
    }
}

impl ToTokens for Data<&Entry> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.0.summary;
        let ident = utils::ident(self.0.name.to_case(Case::Pascal));
        let value = &self.0.value;

        tokens.extend(quote! {
            #[doc = #docs]
            #ident = #value
        });
    }
}

impl ToTokens for Data<&Arg> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.0.summary;
        let ident = utils::ident(&self.0.name);

        let enum_kind = self.0.enum_kind.as_ref().map(|kind| {
            let mut parts = kind.split(".");
            let first = parts.next().unwrap();
            match parts.next() {
                None => utils::ident(first.to_case(Case::Pascal)).into_token_stream(),
                Some(second) => {
                    let interface = utils::ident(first);
                    let ident = utils::ident(second.to_case(Case::Pascal));
                    quote! { #interface::#ident }
                }
            }
        });

        let interface = match &self.0.interface {
            Some(interface) => utils::ident(interface.to_case(Case::Pascal)).into_token_stream(),
            None => quote! { () },
        };

        let mut arg_ty = match self.0.ty {
            ArgType::Fixed => quote! { __protocol::types::Fixed },
            ArgType::String => quote! { String },
            ArgType::Array => quote! { Box<[u8]> },
            ArgType::Fd => quote! { ::std::os::fd::OwnedFd },
            ArgType::Object => quote! { __protocol::types::ObjId<#interface> },
            ArgType::NewId => quote! { __protocol::types::NewId<#interface> },
            ArgType::Int => enum_kind.unwrap_or_else(|| quote! { i32 }),
            ArgType::Uint => enum_kind.unwrap_or_else(|| quote! { u32 }),
        };

        if self.0.allow_null {
            arg_ty = quote! { Option<#arg_ty> };
        }

        tokens.extend(quote! {
            #[doc = #docs]
            #ident: #arg_ty
        });
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
