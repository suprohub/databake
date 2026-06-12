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

    // Collect variant information
    struct VariantInfo {
        ident: syn::Ident,
        fields: Vec<syn::Ident>, // binding names
        field_kind: FieldsKind,
    }
    enum FieldsKind {
        Named,
        Unnamed,
        Unit,
    }

    let variants: Vec<_> = structure
        .variants()
        .iter()
        .map(|v| {
            let ident = v.ast().ident.clone();
            let (fields, kind) = match &v.ast().fields {
                Fields::Named(f) => (
                    f.named.iter().map(|f| f.ident.clone().unwrap()).collect(),
                    FieldsKind::Named,
                ),
                Fields::Unnamed(f) => (
                    (0..f.unnamed.len())
                        .map(|i| syn::Ident::new(&format!("__field_{}", i), ident.span()))
                        .collect(),
                    FieldsKind::Unnamed,
                ),
                Fields::Unit => (Vec::new(), FieldsKind::Unit),
            };
            VariantInfo {
                ident,
                fields,
                field_kind: kind,
            }
        })
        .collect();

    // Helper to build a pattern from a list of idents
    let make_pattern = |fields: &[syn::Ident], kind: &FieldsKind| -> TokenStream2 {
        match kind {
            FieldsKind::Named => {
                let names = fields;
                quote! { { #(#names),* } }
            }
            FieldsKind::Unnamed => {
                let names = fields;
                quote! { ( #(#names),* ) }
            }
            FieldsKind::Unit => quote! {},
        }
    };

    let bake_arms = variants.iter().map(|v| {
        let pattern = make_pattern(&v.fields, &v.field_kind);
        let idents = &v.fields;
        let variant_ident = &v.ident;

        // Recursive bake calls
        let bakes = idents.iter().map(|id| {
            quote! { let #id = #id.bake(env); }
        });

        // Constructor
        let constructor = match &v.field_kind {
            FieldsKind::Named => {
                let fields = idents.iter().map(|id| quote! { #id: #id });
                quote! { #path::#variant_ident #pattern { #(#fields),* } }
            }
            FieldsKind::Unnamed => {
                let fields = idents.iter().map(|id| quote! { #id });
                quote! { #path::#variant_ident(#(#fields),*) }
            }
            FieldsKind::Unit => {
                quote! { #path::#variant_ident }
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
                    match *self {
                        #(#bake_arms)*
                    }
                }
            }
        }
    };

    // ---------- BakeSize impl ----------
    let size_arms = variants.iter().map(|v| {
        let pattern = make_pattern(&v.fields, &v.field_kind);
        let idents = &v.fields;
        let sizes = idents.iter().map(|id| {
            quote! { #id.borrows_size() }
        });
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
                    match *self {
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