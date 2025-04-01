use std::{env, fs::File, io::BufReader, path::PathBuf};

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use serde::Deserialize;
use syn::{Ident, LitStr, parse_macro_input};

pub fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let lit_str = parse_macro_input!(input as LitStr);
    let root_path: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
    match File::open(root_path.join(lit_str.value())) {
        Err(err) => {
            return syn::Error::new(lit_str.span(), err.to_string())
                .into_compile_error()
                .into();
        }
        Ok(file) => match quick_xml::de::from_reader::<_, Protocol>(BufReader::new(file)) {
            Ok(protocol) => protocol.into_token_stream().into(),
            Err(err) => syn::Error::new(lit_str.span(), err.to_string())
                .into_compile_error()
                .into(),
        },
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

impl ToTokens for Protocol {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = Ident::new(&self.name, Span::call_site());
        let interfaces = &self.interfaces;
        tokens.extend(quote! {
            pub mod #ident {
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
    pub fn doc_text(&self) -> &str {
        self.text.as_ref().unwrap_or(&self.summary)
    }
}

#[derive(Debug, Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub version: u32,
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
        let docs = self.description.doc_text();
        let ident = Ident::new(&self.name, Span::call_site());
        let requests = &self.requests;
        let events = &self.events;
        let enums = &self.enums;
        tokens.extend(quote! {
            #[doc = #docs]
            pub mod #ident {
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
        let docs = self.description.doc_text();
        let request_name = format!("{}Request", self.name.to_case(Case::Pascal));
        let request_ident = Ident::new(&request_name, Span::call_site());
        let args = &self.args;
        tokens.extend(quote! {
            #[doc = #docs]
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
    #[serde(rename = "@summary")]
    pub summary: String,
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let docs = &self.summary;
        let arg_ident = Ident::new(&self.name, Span::call_site());
        let arg_ty = &self.ty;
        tokens.extend(quote! {
            #[doc = #docs]
            #arg_ident: #arg_ty
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
            ArgType::Fd => quote! {()},
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
        let docs = self.description.doc_text();
        let event_name = format!("{}Event", self.name.to_case(Case::Pascal));
        let event_ident = Ident::new(&event_name, Span::call_site());
        let args = &self.args;
        tokens.extend(quote! {
            #[doc = #docs]
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
        let docs = self.description.doc_text();
        let enum_ident = Ident::new(&self.name.to_case(Case::Pascal), Span::call_site());
        let entries = &self.entries;
        tokens.extend(quote! {
            #[doc = #docs]
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
            false => Ident::new(s.as_ref(), Span::call_site()),
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
