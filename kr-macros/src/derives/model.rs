use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

use crate::derives::PartialAttr;

pub fn expand_partial_model(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse_macro_input!(input as DeriveInput);
    let fields = match &input.data {
        syn::Data::Struct(s) => &s.fields,
        _ => {
            return syn::Error::new_spanned(&input.ident, "Model can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // 解析所有 #[model(...)]
    let mut generated: Vec<TokenStream2> = Vec::new();
    for attr in &input.attrs {
        if attr.path().is_ident("model") {
            match attr.parse_args::<PartialAttr>() {
                Ok(p) => {
                    let target_ident = &p.target;

                    // 根据 include/exclude 模式筛选字段
                    let keep_fields: Vec<_> = fields
                        .iter()
                        .filter(|f| {
                            let ident = f.ident.as_ref().unwrap();
                            if p.exclude {
                                !p.fields.iter().any(|ex| ex == ident)
                            } else {
                                p.fields.iter().any(|ex| ex == ident)
                            }
                        })
                        .collect();

                    // 生成字段定义（保留属性）
                    let gen_fields = keep_fields.iter().map(|f| {
                        let ident = f.ident.as_ref().unwrap();
                        let ty = &f.ty;
                        let attrs = &f.attrs;
                        quote! {
                            #(#attrs)*
                            pub #ident: #ty
                        }
                    });

                    // 合并 derives: 默认(sqlx::FromRow) + 用户自定义
                    let mut derives = Vec::new();
                    derives.push(syn::parse_quote!(sqlx::FromRow));
                    for d in p.derives {
                        derives.push(d);
                    }
                    let derive_attr = quote! {
                        #[derive(#(#derives),*)]
                    };

                    generated.push(quote! {
                        #derive_attr
                        pub struct #target_ident {
                            #(#gen_fields,)*
                        }
                    });
                }
                Err(e) => return e.to_compile_error().into(),
            }
        }
    }
    quote! { #(#generated)* }.into()
}
