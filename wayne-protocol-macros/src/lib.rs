use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod protocol;

#[proc_macro]
pub fn protocol(input: TokenStream) -> TokenStream {
    match parse_macro_input!(input as protocol::Context).load() {
        Ok(generator) => generator.into_token_stream().into(),
        Err(err) => return err.into_compile_error().into(),
    }
}
