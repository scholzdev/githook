use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ImplItem, ItemImpl, ReturnType, Type, FnArg, PatType};

#[proc_macro_attribute]
pub fn callable_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let self_ty = &impl_block.self_ty;
    
    let mut property_methods = Vec::new();
    let mut callable_methods = Vec::new();
    
    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            let has_property = method.attrs.iter().any(|attr| attr.path().is_ident("property"));
            let has_method = method.attrs.iter().any(|attr| attr.path().is_ident("method"));
            
            if has_property || has_method {
                let method_name = &method.sig.ident;
                let method_name_str = method_name.to_string();
                
                let return_type = match &method.sig.output {
                    ReturnType::Default => parse_quote!(()),
                    ReturnType::Type(_, ty) => (**ty).clone(),
                };
                
                let param_count = method.sig.inputs.iter()
                    .filter(|arg| !matches!(arg, FnArg::Receiver(_)))
                    .count();
                
                if has_property {
                    if param_count > 0 {
                        panic!("#[property] methods cannot have parameters: {}", method_name_str);
                    }
                    
                    let match_arm = generate_conversion(&return_type, quote! { self.#method_name() });
                    property_methods.push(quote! {
                        #method_name_str => #match_arm
                    });
                } else if has_method {
                    if param_count == 0 {
                        let match_arm = generate_conversion(&return_type, quote! { self.#method_name() });
                        callable_methods.push(quote! {
                            #method_name_str => {
                                if !args.is_empty() {
                                    bail!(concat!(#method_name_str, "() expects no arguments, got {}"), args.len());
                                }
                                #match_arm
                            }
                        });
                    } else {
                        // Extract parameter types
                        let params: Vec<_> = method.sig.inputs.iter()
                            .filter_map(|arg| {
                                if let FnArg::Typed(pat_type) = arg {
                                    Some((*pat_type.ty).clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        
                        // Generate arg conversions
                        let arg_conversions: Vec<_> = params.iter().enumerate()
                            .map(|(i, ty)| {
                                generate_arg_conversion(ty, i)
                            })
                            .collect();
                        
                        // Generate arg names for method call
                        let arg_names: Vec<_> = (0..params.len())
                            .map(|i| {
                                let ident = syn::Ident::new(&format!("arg{}", i), proc_macro2::Span::call_site());
                                quote! { #ident }
                            })
                            .collect();
                        
                        let match_arm = generate_conversion(&return_type, quote! { self.#method_name(#(#arg_names),*) });
                        
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
                
                method.attrs.retain(|attr| !attr.path().is_ident("property") && !attr.path().is_ident("method"));
            }
        }
    }
    
    let call_property = quote! {
        pub fn call_property<V>(&self, property_name: &str) -> anyhow::Result<V> 
        where V: From<bool> + From<f64> + From<String>
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
        where V: From<bool> + From<f64> + From<String>
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

fn generate_conversion(return_type: &Type, call_expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if is_bool_type(return_type) {
        quote! { Ok(V::from(#call_expr)) }
    } else if is_number_type(return_type) {
        quote! { Ok(V::from(#call_expr as f64)) }
    } else if is_string_type(return_type) {
        quote! { Ok(V::from(#call_expr)) }
    } else {
        quote! { Ok(V::from(#call_expr)) }
    }
}

fn generate_arg_conversion(_ty: &Type, index: usize) -> proc_macro2::TokenStream {
    let idx = syn::Index::from(index);
    let arg_name = syn::Ident::new(&format!("arg{}", index), proc_macro2::Span::call_site());
    
    // All args come in as &str, we just pass them through
    // Type conversion happens at the method boundary
    quote! {
        let #arg_name = args[#idx];
    }
}

fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

fn is_number_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return matches!(ident.as_str(), "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
                                            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
                                            "f32" | "f64");
        }
    }
    false
}

fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

fn is_str_ref_type(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        if let Type::Path(type_path) = &*type_ref.elem {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "str";
            }
        }
    }
    false
}