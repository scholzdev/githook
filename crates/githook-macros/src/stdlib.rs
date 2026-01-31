use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Lit, Meta};


#[proc_macro_attribute]
pub fn export_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    
    let attr_parser = syn::parse_macro_input!(attr as syn::AttributeArgs);
    
    let mut module = String::from("default");
    let mut doc = String::new();
    
    for arg in attr_parser {
        if let syn::NestedMeta::Meta(Meta::NameValue(nv)) = arg {
            let path = nv.path.get_ident().unwrap().to_string();
            if let Lit::Str(lit_str) = nv.lit {
                match path.as_str() {
                    "module" => module = lit_str.value(),
                    "doc" => doc = lit_str.value(),
                    _ => {}
                }
            }
        }
    }
    
    let expanded = quote! {
        #input_fn
        #[::githook_stdlib_gen::inventory::submit]
        fn #fn_name() -> ::githook_stdlib_gen::MacroEntry {
            ::githook_stdlib_gen::MacroEntry {
                module: #module,
                name: #fn_name_str,
                doc: #doc,
                generator: || #fn_name(),
            }
        }
    };
    
    TokenStream::from(expanded)
}
