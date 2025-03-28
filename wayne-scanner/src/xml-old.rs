use std::{
    env,
    io::BufRead,
    path::{Path, PathBuf},
};

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use quote::{ToTokens, quote};
use syn::{
    Ident, LitStr,
    parse::{Parse, ParseStream},
};

pub struct XMLError {
    inner: anyhow::Error,
    location: u64,
}

pub struct File {
    protocol: Protocol,
}

impl ToTokens for File {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.protocol.to_tokens(tokens);
    }
}

impl Parse for File {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path_lit = input.parse::<LitStr>()?;
        let root: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
        Self::load(&root.join(path_lit.value()))
            .map_err(|e| syn::Error::new(path_lit.span(), format!("Invalid XML: {e}")))
    }
}

impl File {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let mut reader = Reader::from_file(path)?;
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer)? {
                Event::Eof => return Err(anyhow::Error::msg("no un-empty `protocol` tag found")),
                Event::Start(start) if start.name().as_ref() == b"protocol" => {
                    return Ok(File {
                        protocol: Protocol::build(&mut reader)?,
                    });
                }
                _ => continue,
            }
        }
    }
}

pub struct Protocol {
    interfaces: Vec<Interface>,
}

impl ToTokens for Protocol {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for interface in &self.interfaces {
            interface.to_tokens(tokens);
        }
    }
}

impl Protocol {
    pub fn build<R: BufRead>(reader: &mut Reader<R>) -> anyhow::Result<Self> {
        let mut buffer = Vec::new();
        let mut interfaces = Vec::new();

        loop {
            match reader.read_event_into(&mut buffer)? {
                Event::End(end) if end.name().as_ref() == b"protocol" => break,
                Event::Eof => return Err(anyhow::Error::msg("`protocol` tag is unclosed")),
                Event::Start(start) => match start.name().as_ref() {
                    b"interface" => interfaces.push(Interface::build(start, reader)?),
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(Self { interfaces })
    }
}

pub struct Interface {
    name: String,
    version: u32,
    docs: Option<String>,
}

impl ToTokens for Interface {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let version = &self.version;
        let name = Ident::new(&self.name.to_case(Case::Pascal), Span::call_site());
        let docs = self.docs.as_ref().map(|text| quote! { #[doc = #text] });
        tokens.extend(quote! {
            #docs
            pub struct #name {
                _private: (),
            }

            impl #name {
                const VERSION: u32 = #version;
            }
        });
    }
}

impl Interface {
    pub fn build<R: BufRead>(start: BytesStart, reader: &mut Reader<R>) -> anyhow::Result<Self> {
        let name = match start.try_get_attribute("name")? {
            Some(name) => String::from_utf8_lossy(&name.value).to_string(),
            _ => {
                return Err(anyhow::Error::msg(
                    "Failed to find `name` attribute for interface",
                ));
            }
        };

        let version = match start.try_get_attribute("version")? {
            Some(version) => String::from_utf8_lossy(&version.value).parse::<u32>()?,
            None => {
                return Err(anyhow::Error::msg(
                    "Failed to find `name` attribute for interface",
                ));
            }
        };

        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer)? {
                Event::Eof => return Err(anyhow::Error::msg("`interface` tag is unclosed")),
                Event::Start(start) => match start.name().as_ref() {
                    b"description" => {}
                    _ => {}
                },
                Event::End(end) => match end.name().as_ref() {
                    b"interface" => break,
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(Self {
            name,
            version,
            docs: None,
        })
    }
}
