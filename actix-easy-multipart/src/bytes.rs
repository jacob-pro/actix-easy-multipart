//! Reads a field into memory.
use crate::{field_mime, Error, FieldReader, Limits};
use actix_multipart::Field;
use actix_web::HttpRequest;
use bytes::BytesMut;
use futures_core::future::LocalBoxFuture;
use futures_util::{FutureExt, TryStreamExt};
use mime::Mime;

/// Read the field into memory.
#[derive(Debug)]
pub struct Bytes {
    /// The data.
    pub data: bytes::Bytes,
    /// The value of the `content-type` header.
    pub content_type: Option<Mime>,
    /// The `filename` value in the `content-disposition` header.
    pub file_name: Option<String>,
}

impl<'t> FieldReader<'t> for Bytes {
    type Future = LocalBoxFuture<'t, Result<Self, Error>>;

    fn read_field(_: &'t HttpRequest, mut field: Field, limits: &'t mut Limits) -> Self::Future {
        async move {
            let mut data = BytesMut::new();
            while let Some(chunk) = field.try_next().await? {
                limits.try_consume_limits(chunk.len(), true)?;
                data.extend(chunk);
            }
            Ok(Bytes {
                data: data.freeze(),
                content_type: field_mime(&field),
                file_name: field
                    .content_disposition()
                    .get_filename()
                    .map(str::to_owned),
            })
        }
        .boxed_local()
    }
}
