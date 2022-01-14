extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{PathArguments, Type};

#[proc_macro_derive(FromMultipart)]
pub fn impl_from_multipart(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let name = &ast.ident;
    let str = match &ast.data {
        syn::Data::Struct(s) => s,
        _ => panic!("This trait can only be derived for a struct"),
    };
    let fields = match &str.fields {
        syn::Fields::Named(n) => n,
        _ => panic!("This trait can only be derived for a struct"),
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
        let last = &mut x.path.segments.last_mut().unwrap();
        last.value_mut().arguments = PathArguments::None;
        fields_vec_innards.extend(quote!(
            #name: #x::get_from_multiparts(&mut value, stringify!(#name))?,
        ));
    }

    let gen = quote! {
        impl std::convert::TryFrom<actix_easy_multipart::Multiparts> for #name {

            type Error = actix_easy_multipart::deserialize::Error;

            fn try_from(mut value: actix_easy_multipart::Multiparts) -> Result<Self, Self::Error> {
                use actix_easy_multipart::deserialize::RetrieveFromMultiparts;
                use actix_easy_multipart::deserialize::RetrieveFromMultipartsExt;
                let x = Self {
                    #fields_vec_innards
                };
                Ok(x)
            }
        }
    };
    gen.into()
}
