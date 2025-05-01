use std::{env, fs::File, io::BufReader, path::PathBuf, str::Lines};

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use serde::Deserialize;
use syn::{
    Ident, LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

pub struct Context {
    crate_path: Path,
    file_path: LitStr,
}

impl Context {
    pub fn load(&self) -> syn::Result<Generator> {
        let root_path: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
        match File::open(root_path.join(self.file_path.value())) {
            Err(err) => Err(syn::Error::new(self.file_path.span(), err.to_string())),
            Ok(file) => match quick_xml::de::from_reader::<_, Protocol>(BufReader::new(file)) {
                Err(err) => Err(syn::Error::new(self.file_path.span(), err.to_string())),
                Ok(protocol) => Ok(Generator {
                    crate_path: &self.crate_path,
                    protocol,
                }),
            },
        }
    }
}

impl Parse for Context {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let crate_path = input.parse::<Path>()?;
        let _ = input.parse::<Token![,]>()?;
        let file_path = input.parse::<LitStr>()?;
        Ok(Self {
            crate_path,
            file_path,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "protocol")]
pub struct Protocol {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "interface")]
    pub interfaces: Vec<Interface>,
}

pub struct Generator<'a> {
    crate_path: &'a Path,
    protocol: Protocol,
}

impl ToTokens for Generator<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let crate_path = self.crate_path;
        let protocol = Ident::new(&self.protocol.name, Span::call_site());
        let interfaces = &self.protocol.interfaces;
        tokens.extend(quote! {
            pub mod #protocol {
                use #crate_path::*;
                #(#interfaces)*
            }
        });
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Description {
    #[serde(rename = "@summary")]
    pub summary: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

impl Description {
    pub fn doc_lines(&self) -> Lines {
        self.text.as_ref().unwrap_or(&self.summary).lines()
    }
}

#[derive(Debug, Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub _version: u32,
    pub description: Description,
    #[serde(default, rename = "request")]
    pub requests: Vec<Request>,
    #[serde(default, rename = "event")]
    pub events: Vec<Event>,
    #[serde(default, rename = "enum")]
    pub enums: Vec<Enum>,
}

impl ToTokens for Interface {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident_mod = Ident::new(&self.name, Span::call_site());
        let ident_struct = Ident::new(&self.name.to_case(Case::Pascal), Span::call_site());
        let requests = &self.requests;
        let events = &self.events;
        let enums = &self.enums;

        let docs = self.description.doc_lines().map(|doc| {
            let doc = doc.trim();
            quote! {
                #[doc = #doc]
            }
        });
        tokens.extend(quote! {
            #(#docs)*
            pub struct #ident_struct(());
            pub mod #ident_mod {
                use super::*;
                #(#requests)*
                #(#events)*
                #(#enums)*
            }
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(rename = "@name")]
    pub name: String,
    pub description: Description,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

impl ToTokens for Request {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let request_name = format!("{}Request", self.name.to_case(Case::Pascal));
        let request_ident = Ident::new(&request_name, Span::call_site());
        let args = &self.args;

        let docs = self.description.doc_lines().map(|doc| {
            let doc = doc.trim();
            quote! {
                #[doc = #doc]
            }
        });
        tokens.extend(quote! {
            #(#docs)*
            pub struct #request_ident {
                #(#args,)*
            }
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct Arg {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub ty: ArgType,
    #[serde(rename = "@interface")]
    pub interface: Option<String>,
    #[serde(rename = "@enum")]
    pub enum_kind: Option<String>,
    #[serde(rename = "@allow-null")]
    pub allow_null: Option<String>,
    #[serde(rename = "@summary")]
    pub summary: String,
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.summary;
        let arg_ident = Ident::new(&self.name, Span::call_site());

        let enum_kind = self.enum_kind.as_ref().map(|kind| {
            let mut parts = kind.split(".");
            let first = parts.next().unwrap();
            match parts.next() {
                Some(second) => {
                    let interface = Ident::new(first, Span::call_site());
                    let ident_name = second.to_case(Case::Pascal);
                    let ident = Ident::new(&ident_name, Span::call_site());
                    quote! { #interface::#ident }
                }
                None => {
                    let ident_name = first.to_case(Case::Pascal);
                    let ident = Ident::new(&ident_name, Span::call_site());
                    ident.into_token_stream()
                }
            }
        });

        let interface = match &self.interface {
            None => quote! { () },
            Some(interface) => {
                let ident_name = interface.to_case(Case::Pascal);
                let ident = Ident::new(&ident_name, Span::call_site());
                quote! { #ident }
            }
        };

        let arg_ty = match self.ty {
            ArgType::Fixed => quote! { types::Fixed },
            ArgType::String => quote! { String },
            ArgType::Array => quote! { Box<[u8]> },
            ArgType::Fd => quote! { ::std::os::fd::OwnedFd },
            ArgType::Object => quote! { types::ObjectId<#interface> },
            ArgType::NewId => quote! { types::NewId<#interface> },
            ArgType::Int => enum_kind.unwrap_or_else(|| quote! { i32 }),
            ArgType::Uint => enum_kind.unwrap_or_else(|| quote! { u32 }),
        };

        let arg_ty = match self.allow_null {
            Some(_) => quote! { Option<#arg_ty> },
            None => arg_ty,
        };

        tokens.extend(quote! {
            #[doc = #docs]
            pub #arg_ident: #arg_ty
        });
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    Int,
    Uint,
    Fixed,
    String,
    Object,
    NewId,
    Array,
    Fd,
}

impl ToTokens for ArgType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ArgType::Int => quote! {i32},
            ArgType::Uint => quote! {u32},
            ArgType::Fixed => quote! {()},
            ArgType::String => quote! {String},
            ArgType::Object => quote! {u32},
            ArgType::NewId => quote! {u32},
            ArgType::Array => quote! {Box<[u8]>},
            ArgType::Fd => quote! {::std::os::fd::OwnedFd},
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "@name")]
    pub name: String,
    pub description: Description,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

impl ToTokens for Event {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let event_name = format!("{}Event", self.name.to_case(Case::Pascal));
        let event_ident = Ident::new(&event_name, Span::call_site());
        let args = &self.args;

        let docs = self.description.doc_lines().map(|doc| {
            let doc = doc.trim();
            quote! {
                #[doc = #doc]
            }
        });
        tokens.extend(quote! {
            #(#docs)*
            pub struct #event_ident {
                #(#args,)*
            }
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct Enum {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default)]
    pub description: Description,
    #[serde(rename = "entry")]
    pub entries: Vec<Entry>,
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let enum_ident = Ident::new(&self.name.to_case(Case::Pascal), Span::call_site());
        let entries = &self.entries;

        let docs = self.description.doc_lines().map(|doc| {
            let doc = doc.trim();
            quote! {
                #[doc = #doc]
            }
        });
        tokens.extend(quote! {
            #(#docs)*
            #[repr(u32)]
            pub enum #enum_ident {
                #(#entries,)*
            }
        });
    }
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value", deserialize_with = "utils::parse_hex")]
    pub value: u32,
    #[serde(default, rename = "@summary")]
    pub summary: String,
}

impl ToTokens for Entry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.summary;
        let entry_name = utils::pascal(&self.name);
        let entry_ident = utils::ident(entry_name);
        let value = &self.value;
        tokens.extend(quote! {
            #[doc = #docs]
            #entry_ident = #value
        });
    }
}

mod utils {
    use convert_case::{Case, Casing};
    use proc_macro2::Span;
    use serde::{Deserialize, Deserializer};
    use syn::Ident;

    pub fn ident(s: impl AsRef<str>) -> Ident {
        let s = s.as_ref();
        match s.starts_with(|c: char| c.is_numeric()) {
            true => Ident::new(&format!("_{s}"), Span::call_site()),
            false => Ident::new(s, Span::call_site()),
        }
    }

    pub fn pascal(s: impl AsRef<str>) -> String {
        s.as_ref().to_case(Case::Pascal)
    }

    pub fn parse_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let string = String::deserialize(deserializer)?;
        match string.starts_with("0x") {
            false => string
                .parse::<u32>()
                .map_err(|err| serde::de::Error::custom(err.to_string())),
            true => u32::from_str_radix(&string[2..], 16)
                .map_err(|err| serde::de::Error::custom(err.to_string())),
        }
    }
}
