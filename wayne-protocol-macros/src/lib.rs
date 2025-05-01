use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod protocol;

#[proc_macro]
pub fn protocol(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as protocol::Generator)
        .into_token_stream()
        .into()
}
