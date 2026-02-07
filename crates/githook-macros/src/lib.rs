//! # githook-macros
//!
//! Procedural macros for the Githook scripting language.
//!
//! Provides `#[callable_impl]` for auto-generating `call_property()`
//! and `call_method()` dispatchers, `#[builtin]` for registering
//! built-in functions, and `#[export_macro]` for exposing Rust
//! functions as Githook macros.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    FnArg, ImplItem, ItemFn, ItemImpl, Lit, Meta, ReturnType, Type, parse_macro_input, parse_quote,
};

/// Attribute for attaching documentation metadata to context methods.
///
/// Used by the LSP hover provider to surface help text.
#[proc_macro_attribute]
pub fn docs(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Registers a function as a Githook built-in using `inventory`.
#[proc_macro_attribute]
pub fn builtin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let expanded = quote! {
        #[allow(dead_code)]
        #input_fn

        inventory::submit! {
            crate::builtins::BuiltinEntry {
                name: #fn_name_str,
                func: #fn_name,
            }
        }
    };

    TokenStream::from(expanded)
}

/// Exports a Rust function as a callable Githook macro.
#[proc_macro_attribute]
pub fn export_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_body = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    let attr_parser = syn::parse::Parser::parse(
        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
        attr,
    )
    .unwrap();

    let mut module = String::from("default");

    for meta in attr_parser {
        if let Meta::NameValue(nv) = meta
            && let Some(ident) = nv.path.get_ident()
            && ident == "module"
            && let syn::Expr::Lit(expr_lit) = &nv.value
            && let Lit::Str(lit_str) = &expr_lit.lit
        {
            module = lit_str.value();
        }
    }

    let mut docs_str = None;
    for attr in fn_attrs {
        if attr.path().is_ident("__githook_docs")
            && let Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(expr_lit) = &nv.value
            && let Lit::Str(lit_str) = &expr_lit.lit
        {
            docs_str = Some(lit_str.value());
            break;
        }
    }

    let filtered_attrs: Vec<_> = fn_attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("__githook_docs"))
        .collect();

    let docs_value = if let Some(d) = docs_str {
        quote! { Some(#d) }
    } else {
        quote! { None }
    };

    let expanded = quote! {
        #(#filtered_attrs)*
        #fn_vis fn #fn_name() -> crate::macro_def::Macro {
            #fn_body
        }

        crate::inventory::submit! {
            crate::MacroEntry {
                module: #module,
                generator: #fn_name,
                docs: #docs_value,
            }
        }
    };

    TokenStream::from(expanded)
}

/// Auto-generates `call_property()` and `call_method()` dispatchers for a context impl block.
///
/// Methods annotated `#[property]` are dispatched by `call_property(name)`.
/// Methods annotated `#[method]` are dispatched by `call_method(name, args)`.
#[proc_macro_attribute]
pub fn callable_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let self_ty = &impl_block.self_ty;

    let mut property_methods = Vec::new();
    let mut callable_methods = Vec::new();

    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            let has_property = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("property"));
            let has_method = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("method"));

            if has_property || has_method {
                let method_name = &method.sig.ident;
                let method_name_str = method_name.to_string();

                let return_type = match &method.sig.output {
                    ReturnType::Default => parse_quote!(()),
                    ReturnType::Type(_, ty) => (**ty).clone(),
                };

                let param_count = method
                    .sig
                    .inputs
                    .iter()
                    .filter(|arg| !matches!(arg, FnArg::Receiver(_)))
                    .count();

                if has_property {
                    if param_count > 0 {
                        panic!(
                            "#[property] methods cannot have parameters: {}",
                            method_name_str
                        );
                    }

                    let match_arm =
                        generate_conversion(&return_type, quote! { self.#method_name() });
                    property_methods.push(quote! {
                        #method_name_str => #match_arm
                    });
                } else if has_method {
                    if param_count == 0 {
                        let match_arm =
                            generate_conversion(&return_type, quote! { self.#method_name() });
                        callable_methods.push(quote! {
                            #method_name_str => {
                                if !args.is_empty() {
                                    bail!(concat!(#method_name_str, "() expects no arguments, got {}"), args.len());
                                }
                                #match_arm
                            }
                        });
                    } else {
                        let params: Vec<_> = method
                            .sig
                            .inputs
                            .iter()
                            .filter_map(|arg| {
                                if let FnArg::Typed(pat_type) = arg {
                                    Some((*pat_type.ty).clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let arg_conversions: Vec<_> = params
                            .iter()
                            .enumerate()
                            .map(|(i, ty)| generate_arg_conversion(ty, i))
                            .collect();

                        let arg_names: Vec<_> = (0..params.len())
                            .map(|i| {
                                let ident = syn::Ident::new(
                                    &format!("arg{}", i),
                                    proc_macro2::Span::call_site(),
                                );
                                quote! { #ident }
                            })
                            .collect();

                        let match_arm = generate_conversion(
                            &return_type,
                            quote! { self.#method_name(#(#arg_names),*) },
                        );

                        callable_methods.push(quote! {
                            #method_name_str => {
                                if args.len() != #param_count {
                                    bail!(concat!(#method_name_str, "() expects ", #param_count, " argument(s), got {}"), args.len());
                                }
                                #(#arg_conversions)*
                                #match_arm
                            }
                        });
                    }
                }

                method.attrs.retain(|attr| {
                    !attr.path().is_ident("property") && !attr.path().is_ident("method")
                });
            }
        }
    }

    let call_property = quote! {
        pub fn call_property<V>(&self, property_name: &str) -> anyhow::Result<V>
        where V: From<bool> + From<f64> + From<String> + From<Vec<String>>
        {
            use anyhow::bail;
            match property_name {
                #(#property_methods,)*
                _ => bail!("Property '{}' not found on {}", property_name, stringify!(#self_ty)),
            }
        }
    };

    let call_method = quote! {
        pub fn call_method<V>(&self, method_name: &str, args: &[&str]) -> anyhow::Result<V>
        where V: From<bool> + From<f64> + From<String> + From<Vec<String>>
        {
            use anyhow::bail;
            match method_name {
                #(#callable_methods)*
                _ => bail!("Method '{}' not found on {}", method_name, stringify!(#self_ty)),
            }
        }
    };

    impl_block.items.push(parse_quote! { #call_property });
    impl_block.items.push(parse_quote! { #call_method });

    TokenStream::from(quote! { #impl_block })
}

fn generate_conversion(
    return_type: &Type,
    call_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if is_number_type(return_type) {
        quote! { Ok(V::from(#call_expr as f64)) }
    } else {
        quote! { Ok(V::from(#call_expr)) }
    }
}

fn generate_arg_conversion(ty: &Type, index: usize) -> proc_macro2::TokenStream {
    let idx = syn::Index::from(index);
    let arg_name = syn::Ident::new(&format!("arg{}", index), proc_macro2::Span::call_site());

    if is_number_type(ty) {
        quote! {
            let #arg_name: f64 = args[#idx].parse()
                .map_err(|_| anyhow::anyhow!("Failed to parse argument {} as number", #idx))?;
        }
    } else if is_bool_type(ty) {
        quote! {
            let #arg_name: bool = args[#idx].parse()
                .map_err(|_| anyhow::anyhow!("Failed to parse argument {} as bool", #idx))?;
        }
    } else {
        quote! {
            let #arg_name = args[#idx];
        }
    }
}

fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "bool";
    }
    false
}

fn is_number_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let ident = segment.ident.to_string();
        return matches!(
            ident.as_str(),
            "u8" | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "f32"
                | "f64"
        );
    }
    false
}
