extern crate proc_macro;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, PathArguments, Type};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(from_multipart), default)]
struct FromMultipartAttrs {
    deny_extra_parts: bool,
}

#[proc_macro_derive(FromMultipart, attributes(from_multipart))]
pub fn impl_from_multipart(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    let attrs: FromMultipartAttrs = match FromMultipartAttrs::from_derive_input(&input) {
        Ok(attrs) => attrs,
        Err(e) => return e.write_errors().into(),
    };

    let mut fields_vec_innards = quote!();
    for field in fields.named.iter() {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let p = match ty {
            Type::Path(ref p) => p,
            _ => panic!("Field must be a TypePath"),
        };
        let mut x = p.clone();
        let last = x.path.segments.last_mut().unwrap();
        last.arguments = PathArguments::None;
        fields_vec_innards.extend(quote!(
            #name: #x::from_fields(
                form.remove(stringify!(#name)).unwrap_or_default(),
                &cfg,
                stringify!(#name)
            )?,
        ));
    }

    let deny_extra_parts: TokenStream = attrs.deny_extra_parts.to_string().parse().unwrap();

    let deny_extras_check = if attrs.deny_extra_parts {
        quote!(if let Some(fields) = form.iter().next() {
            return Err(Self::Error::UnexpectedPart(fields.0.to_owned()));
        })
    } else {
        quote!()
    };

    let gen = quote! {
        impl std::convert::TryFrom<actix_easy_multipart::load::GroupedFields> for #name {

            type Error = actix_easy_multipart::deserialize::Error;

            fn try_from(mut form: actix_easy_multipart::load::GroupedFields) -> Result<Self, Self::Error> {
                use actix_easy_multipart::deserialize::FromField;
                use actix_easy_multipart::deserialize::FromFieldExt;
                let cfg = actix_easy_multipart::deserialize::FromFieldConfig {
                    deny_extra_parts: #deny_extra_parts
                };
                let target = Self {
                    #fields_vec_innards
                };
                #deny_extras_check
                Ok(target)
            }
        }
    };
    gen.into()
}
