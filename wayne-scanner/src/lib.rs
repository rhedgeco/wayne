use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod xml;

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as xml::File)
        .into_token_stream()
        .into()
}
