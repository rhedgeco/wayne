use proc_macro::TokenStream;

mod protocol;

#[proc_macro]
pub fn protocol(input: TokenStream) -> TokenStream {
    protocol::generate(input)
}
