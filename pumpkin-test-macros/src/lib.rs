//! Attribute macro for `pumpkin-test` integration tests.

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, parse_macro_input};

/// Turns an `async fn` into a single-threaded Tokio test, replacing
/// `#[tokio::test]`.
///
/// If the function declares a first parameter, a fresh `TestServer` is built
/// (with the optional `seed`, default `0`) and bound to it before the body
/// runs. `TestServer` must be in scope at the call site.
///
/// ```ignore
/// #[pumpkin_test]
/// async fn no_fixture() { /* build your own TestServer */ }
///
/// #[pumpkin_test(seed = 12345)]
/// async fn with_fixture(mut t: TestServer) { /* use t */ }
/// ```
#[proc_macro_attribute]
pub fn pumpkin_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Seed defaults to 0; captured as tokens so negative literals work, since
    // Minecraft world seeds are signed (Java i64).
    let mut seed = quote! { 0 };
    let seed_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("seed") {
            let value: syn::Expr = meta.value()?.parse()?;
            seed = quote! { #value };
            Ok(())
        } else {
            Err(meta.error("unsupported `pumpkin_test` argument (expected `seed = <int>`)"))
        }
    });
    parse_macro_input!(attr with seed_parser);

    let func = parse_macro_input!(item as ItemFn);
    let attrs = &func.attrs;
    let vis = &func.vis;
    let name = &func.sig.ident;
    let statements = &func.block.stmts;

    // If the test takes a parameter, inject a freshly built `TestServer`.
    let setup = match func.sig.inputs.first() {
        Some(FnArg::Typed(arg)) => {
            let binding = &arg.pat;
            quote! { let #binding = TestServer::new(#seed).await; }
        }
        _ => quote! {},
    };

    quote! {
        #(#attrs)*
        #[test]
        #vis fn #name() {
            let runtime = ::tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("failed to build the test runtime");
            runtime.block_on(async move {
                #setup
                #(#statements)*
            });
        }
    }
    .into()
}
