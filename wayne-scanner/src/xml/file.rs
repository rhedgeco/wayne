use std::{env, path::PathBuf};

use proc_macro2::{Span, TokenStream};
use quick_xml::{Reader, events::Event};
use quote::{ToTokens, quote};
use syn::{
    Ident, LitStr,
    parse::{Parse, ParseStream},
};

use super::XmlTag;

pub struct File {
    protocol: XmlTag,
}

impl Parse for File {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path_lit = input.parse::<LitStr>()?;
        let root: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();
        match Reader::from_file(&root.join(path_lit.value())) {
            Err(error) => Err(syn::Error::new(path_lit.span(), error.to_string())),
            Ok(mut reader) => loop {
                match reader.read_event_into(&mut Vec::new()) {
                    Err(error) => return Err(syn::Error::new(path_lit.span(), error.to_string())),
                    Ok(Event::Eof) => {
                        return Err(syn::Error::new(path_lit.span(), "unexpected EOF"));
                    }
                    Ok(Event::Empty(start)) if start.name().as_ref() == b"protocol" => {
                        return Ok(Self {
                            protocol: XmlTag::empty(&start),
                        });
                    }
                    Ok(Event::Start(start)) if start.name().as_ref() == b"protocol" => {
                        match XmlTag::build(&start, &mut reader) {
                            Ok(protocol) => return Ok(Self { protocol }),
                            Err(error) => {
                                return Err(syn::Error::new(path_lit.span(), error.to_string()));
                            }
                        }
                    }
                    Ok(_) => continue,
                }
            },
        }
    }
}

impl ToTokens for File {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Some(tags) = self.protocol.inner_tags("interface") else {
            return;
        };

        for tag in tags {
            generate_interface(tag, tokens);
        }
    }
}

fn generate_interface(tag: &XmlTag, tokens: &mut TokenStream) {
    if tag.name() != "interface" {
        return;
    }

    let Some(name) = tag.get_attr("name") else {
        return;
    };

    let Some(version) = tag.get_attr("version") else {
        return;
    };

    let Ok(version) = version.parse::<u32>() else {
        return;
    };

    let docs = tag.inner_tags("description").map(|tag| {
        let text = tag.first().unwrap().inner_text();
        quote! { #[doc = #text] }
    });

    let ident = Ident::new(name, Span::call_site());
    tokens.extend(quote! {
        #docs
        pub mod #ident {
            const VERSION: u32 = #version;
        }
    });
}
