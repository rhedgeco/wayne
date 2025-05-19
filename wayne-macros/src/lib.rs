use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod protocol;

/// ⚠️ __This is the raw macro implementation for `wayne_protocol::protocol` and is not usually intended to be used on its own.__ ⚠️
///
/// It is preferred to use `wayne_protocol::protocol!(file_path)` instead as the crate path does not need to be specified.
///
/// This macro takes two parameters separated by a comma `protocol!(protocol_path, file_path)` and generates the associated rust structures:
/// - `protocol_path`: the absolute path to the `wayne-protocol` crate.
/// - `file_path`: the path to the protocol xml file relative to the crate root.
#[proc_macro]
pub fn protocol(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as protocol::Generator)
        .into_token_stream()
        .into()
}
