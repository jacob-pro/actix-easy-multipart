extern crate proc_macro;

use darling::{FromDeriveInput, FromField, FromMeta};
use parse_size::parse_size;
use proc_macro2::Ident;
use quote::quote;
use std::collections::HashSet;
use syn::{parse_macro_input, Type};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(multipart), default)]
struct MultipartFormAttrs {
    deny_unknown_fields: bool,
    duplicate_action: DuplicateAction,
}

#[derive(FromMeta)]
enum DuplicateAction {
    Ignore,
    Deny,
    Replace,
}

impl Default for DuplicateAction {
    fn default() -> Self {
        Self::Ignore
    }
}

#[derive(FromField, Default)]
#[darling(attributes(multipart), default)]
struct FieldAttrs {
    rename: Option<String>,
    limit: Option<String>,
}

struct ParsedField<'t> {
    serialization_name: String,
    rust_name: &'t Ident,
    limit: Option<usize>,
    ty: &'t Type,
}

#[proc_macro_derive(MultipartForm, attributes(multipart))]
pub fn impl_multipart_form(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = parse_macro_input!(input);

    let name = &input.ident;
    let str = match &input.data {
        syn::Data::Struct(s) => s,
        _ => panic!("This trait can only be derived for a struct"),
    };
    let fields = match &str.fields {
        syn::Fields::Named(n) => n,
        _ => panic!("This trait can only be derived for a struct"),
    };

    let attrs: MultipartFormAttrs = match MultipartFormAttrs::from_derive_input(&input) {
        Ok(attrs) => attrs,
        Err(e) => return e.write_errors().into(),
    };

    // Parse the field attributes
    let parsed = match fields
        .named
        .iter()
        .map(|field| {
            let rust_name = field.ident.as_ref().unwrap();
            let attrs: FieldAttrs = FieldAttrs::from_field(field)?;
            let serialization_name = attrs.rename.unwrap_or_else(|| rust_name.to_string());

            let limit = attrs.limit.map(|l| {
                parse_size(&l).unwrap_or_else(|_| panic!("Unable to parse limit `{l}`")) as usize
            });

            Ok(ParsedField {
                serialization_name,
                rust_name,
                limit,
                ty: &field.ty,
            })
        })
        .collect::<Result<Vec<_>, darling::Error>>()
    {
        Ok(attrs) => attrs,
        Err(e) => return e.write_errors().into(),
    };

    // Check that field names are unique
    let mut set = HashSet::new();
    for f in &parsed {
        if !set.insert(f.serialization_name.clone()) {
            panic!("Multiple fields named: `{}`", f.serialization_name);
        }
    }

    // Return value when a field name is not supported by the form
    let unknown_field_result = if attrs.deny_unknown_fields {
        quote!(::std::result::Result::Err(
            ::actix_easy_multipart::Error::UnsupportedField(field.name().to_string())
        ))
    } else {
        quote!(::std::result::Result::Ok(()))
    };

    // Value for duplicate action
    let duplicate_action = match attrs.duplicate_action {
        DuplicateAction::Ignore => quote!(::actix_easy_multipart::DuplicateAction::Ignore),
        DuplicateAction::Deny => quote!(::actix_easy_multipart::DuplicateAction::Deny),
        DuplicateAction::Replace => quote!(::actix_easy_multipart::DuplicateAction::Replace),
    };

    // read_field() implementation
    let mut read_field_impl = quote!();
    for field in &parsed {
        let name = &field.serialization_name;
        let ty = &field.ty;
        read_field_impl.extend(quote!(
            #name => ::std::boxed::Box::pin(
                <#ty as ::actix_easy_multipart::FieldGroupReader>::handle_field(req, field, limits, state, #duplicate_action)
            ),
        ));
    }

    // limit() implementation
    let mut limit_impl = quote!();
    for field in &parsed {
        let name = &field.serialization_name;
        if let Some(value) = field.limit {
            limit_impl.extend(quote!(
                #name => ::std::option::Option::Some(#value),
            ));
        }
    }

    // from_state() implementation
    let mut from_state_impl = quote!();
    for field in &parsed {
        let name = &field.serialization_name;
        let rust_name = &field.rust_name;
        let ty = &field.ty;
        from_state_impl.extend(quote!(
            #rust_name: <#ty as ::actix_easy_multipart::FieldGroupReader>::from_state(#name, &mut state)?,
        ));
    }

    let gen = quote! {
        impl ::actix_easy_multipart::MultipartFormTrait for #name {
            fn limit(field_name: &str) -> ::std::option::Option<usize> {
                match field_name {
                    #limit_impl
                    _ => None,
                }
            }

            fn handle_field<'t>(
                req: &'t ::actix_web::HttpRequest,
                field: ::actix_multipart::Field,
                limits: &'t mut ::actix_easy_multipart::Limits,
                state: &'t mut ::actix_easy_multipart::State,
            ) -> ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = ::std::result::Result<(), ::actix_easy_multipart::Error>> + 't>> {
                match field.name() {
                    #read_field_impl
                    _ => return ::std::boxed::Box::pin(::std::future::ready(#unknown_field_result)),
                }
            }

            fn from_state(mut state: ::actix_easy_multipart::State) -> ::std::result::Result<Self, ::actix_easy_multipart::Error> {
                Ok(Self {
                    #from_state_impl
                })
            }

        }
    };
    gen.into()
}
