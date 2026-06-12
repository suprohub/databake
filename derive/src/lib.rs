// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

#![warn(missing_docs)]

//! Custom derives for `Bake`

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    DeriveInput, Fields, Ident, Path, PathSegment, Token,
};
use synstructure::Structure;

#[proc_macro_derive(Bake, attributes(databake))]
pub fn bake_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(bake_derive_impl(&input))
}

fn bake_derive_impl(input: &DeriveInput) -> TokenStream2 {
    let structure = Structure::new(input);

    struct PathAttr(Punctuated<PathSegment, Token![::]>);
    impl Parse for PathAttr {
        fn parse(input: ParseStream<'_>) -> syn::parse::Result<Self> {
            let i: Ident = input.parse()?;
            if i != "path" {
                return Err(input.error(format!("expected token \"path\", found {i:?}")));
            }
            input.parse::<Token![=]>()?;
            Ok(Self(input.parse::<Path>()?.segments))
        }
    }

    let path = input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("databake"))
        .expect("missing databake(path = ...) attribute")
        .parse_args::<PathAttr>()
        .unwrap()
        .0;

    let crate_name = path.iter().next().unwrap();
    let crate_name_str = quote!(#crate_name).to_string();

    let is_enum = matches!(input.data, syn::Data::Enum(_));

    struct FieldSet {
        fields: Vec<syn::Ident>,
        kind: FieldsKind,
    }
    enum FieldsKind {
        Named,
        Unnamed,
        Unit,
    }

    // Build match pattern and return it along with the list of field idents.
    // For enum variants we prefix the pattern with `Self::` to make the path unambiguous.
    let build_arms = |variant_ident: Option<&syn::Ident>, fs: &FieldSet| -> (TokenStream2, Vec<syn::Ident>) {
        let idents = &fs.fields;
        let pattern = match &fs.kind {
            FieldsKind::Named => {
                if let Some(v) = variant_ident {
                    quote! { Self::#v { #(#idents),* } }
                } else {
                    quote! { Self { #(#idents),* } }
                }
            }
            FieldsKind::Unnamed => {
                if let Some(v) = variant_ident {
                    quote! { Self::#v(#(#idents),*) }
                } else {
                    quote! { Self(#(#idents),*) }
                }
            }
            FieldsKind::Unit => {
                if let Some(v) = variant_ident {
                    quote! { Self::#v }
                } else {
                    quote! { Self }
                }
            }
        };
        (pattern, idents.clone())
    };

    // Gather all variants (or the single struct "variant").
    let field_sets: Vec<(Option<syn::Ident>, FieldSet)> = if is_enum {
        structure
            .variants()
            .iter()
            .map(|v| {
                let ident = Some(v.ast().ident.clone());
                let (fields, kind) = match &v.ast().fields {
                    Fields::Named(f) => (
                        f.named.iter().map(|f| f.ident.clone().unwrap()).collect(),
                        FieldsKind::Named,
                    ),
                    Fields::Unnamed(f) => (
                        (0..f.unnamed.len())
                            .map(|i| syn::Ident::new(&format!("__field_{}", i), v.ast().ident.span()))
                            .collect(),
                        FieldsKind::Unnamed,
                    ),
                    Fields::Unit => (Vec::new(), FieldsKind::Unit),
                };
                (ident, FieldSet { fields, kind })
            })
            .collect()
    } else {
        let fs = match &input.data {
            syn::Data::Struct(s) => match &s.fields {
                Fields::Named(f) => FieldSet {
                    fields: f.named.iter().map(|f| f.ident.clone().unwrap()).collect(),
                    kind: FieldsKind::Named,
                },
                Fields::Unnamed(f) => FieldSet {
                    fields: (0..f.unnamed.len())
                        .map(|i| syn::Ident::new(&format!("__field_{}", i), input.ident.span()))
                        .collect(),
                    kind: FieldsKind::Unnamed,
                },
                Fields::Unit => FieldSet {
                    fields: Vec::new(),
                    kind: FieldsKind::Unit,
                },
            },
            _ => unreachable!(),
        };
        vec![(None, fs)]
    };

    let bake_arms = field_sets.iter().map(|(variant_ident, fs)| {
        let (pattern, idents) = build_arms(variant_ident.as_ref(), fs);

        let bakes = idents.iter().map(|id| {
            quote! { let #id = #id.bake(env); }
        });

        let constructor = match &fs.kind {
            FieldsKind::Named => {
                let fields = idents.iter().map(|id| quote! { #id: #id });
                if let Some(v) = variant_ident {
                    quote! { #path::#v { #(#fields),* } }
                } else {
                    quote! { #path::#input { #(#fields),* } }
                }
            }
            FieldsKind::Unnamed => {
                let fields = idents.iter().map(|id| quote! { #id });
                if let Some(v) = variant_ident {
                    quote! { #path::#v(#(#fields),*) }
                } else {
                    quote! { #path::#input(#(#fields),*) }
                }
            }
            FieldsKind::Unit => {
                if let Some(v) = variant_ident {
                    quote! { #path::#v }
                } else {
                    quote! { #path::#input }
                }
            }
        };

        quote! {
            #pattern => {
                #(#bakes)*
                databake::quote! { #constructor }
            }
        }
    });

    let bake_impl = {
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        let ty = &input.ident;
        quote! {
            impl #impl_generics databake::Bake for #ty #ty_generics #where_clause {
                fn bake(&self, env: &databake::CrateEnv) -> databake::TokenStream {
                    env.insert(#crate_name_str);
                    match self {
                        #(#bake_arms)*
                    }
                }
            }
        }
    };

    let size_arms = field_sets.iter().map(|(variant_ident, fs)| {
        let (pattern, idents) = build_arms(variant_ident.as_ref(), fs);
        let sizes = idents.iter().map(|id| quote! { #id.borrows_size() });
        quote! {
            #pattern => {
                0 #(+ #sizes)*
            }
        }
    });

    let size_impl = {
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        let ty = &input.ident;
        quote! {
            impl #impl_generics databake::BakeSize for #ty #ty_generics #where_clause {
                fn borrows_size(&self) -> usize {
                    match self {
                        #(#size_arms)*
                    }
                }
            }
        }
    };

    quote! {
        #bake_impl
        #size_impl
    }
}