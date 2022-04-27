//! Multipart data extractor.

use crate::load::{GroupedFields, Loader};
use crate::{Error, DEFAULT_FILE_LIMIT, DEFAULT_MAX_PARTS, DEFAULT_TEXT_LIMIT};
use actix_multipart::{Multipart, MultipartError};
use actix_web::dev::Payload;
use actix_web::http::StatusCode;
use actix_web::{FromRequest, HttpRequest, ResponseError};
use futures::future::LocalBoxFuture;
use futures::{FutureExt, TryFutureExt};
use std::convert::TryFrom;
use std::ops;
use std::rc::Rc;
use thiserror::Error;

/// Multipart form extractor (`multipart/form-data`).
///
/// Can be used to extract a multipart form from the request body.
///
/// [MultipartFormConfig] allows you to configure extraction process.
///
/// # Example
/// First define a structure to represent the form that implements `FromMultipart` traits.
/// Then use the extractor in your route.
///
/// ```
/// # fn main() {
/// # use actix_easy_multipart_derive::FromMultipart;
/// #[derive(FromMultipart)]
/// struct Upload {
///    description: String,
///    image: File,
/// }
/// # use actix_web::Responder;
/// # use actix_easy_multipart::File;
/// # use actix_easy_multipart::extractor::MultipartForm;
///
/// async fn route(form: MultipartForm<Upload>) -> impl Responder {
///     format!("Received image of size: {}", form.image.size)
/// }
/// # }
/// ```
pub struct MultipartForm<T>(pub T);

impl<T> MultipartForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for MultipartForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for MultipartForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> FromRequest for MultipartForm<T>
where
    T: TryFrom<GroupedFields, Error = Error> + 'static,
{
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = req
            .app_data::<MultipartFormConfig>()
            .cloned()
            .unwrap_or_default();

        let payload = Multipart::new(req.headers(), payload.take());
        let loader = Loader::builder()
            .max_parts(config.max_parts)
            .file_limit(config.file_limit)
            .text_limit(config.text_limit)
            .build();
        loader
            .load_grouped(payload)
            .map(move |res| match res {
                Ok(item) => {
                    let t = T::try_from(item)?;
                    Ok(MultipartForm(t))
                }
                Err(e) => Err(MultipartFormError::Multipart(e)),
            })
            .map_err(move |e| {
                if let Some(err) = config.error_handler {
                    (*err)(e, &req2)
                } else {
                    Self::Error::from(e)
                }
            })
            .boxed_local()
    }
}

/// Configure the behaviour of the [MultipartForm] extractor.
///
/// # Usage
/// Add a [MultipartFormConfig] to your actix app data.
/// ```
/// # use actix_web::web::scope;
/// # use actix_easy_multipart::extractor;
/// scope("/").app_data(
///     extractor::MultipartFormConfig::default().file_limit(25 * 1024 * 1024) // 25 MiB
/// );
/// ```
#[derive(Clone)]
pub struct MultipartFormConfig {
    text_limit: usize,
    file_limit: usize,
    max_parts: usize,
    error_handler: Option<Rc<dyn Fn(MultipartFormError, &HttpRequest) -> actix_web::Error>>,
}

impl MultipartFormConfig {
    /// Change maximum allowed bytes of text in the form.
    pub fn text_limit(mut self, limit: usize) -> Self {
        self.text_limit = limit;
        self
    }

    /// Change maximum allowed bytes for all files in the form.
    pub fn file_limit(mut self, limit: usize) -> Self {
        self.file_limit = limit;
        self
    }

    /// Change maximum allowed parts in the form.
    pub fn max_parts(mut self, max: usize) -> Self {
        self.max_parts = max;
        self
    }

    /// Set custom error handler.
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(MultipartFormError, &HttpRequest) -> actix_web::Error + 'static,
    {
        self.error_handler = Some(Rc::new(f));
        self
    }
}

impl Default for MultipartFormConfig {
    fn default() -> Self {
        Self {
            text_limit: DEFAULT_TEXT_LIMIT,
            file_limit: DEFAULT_FILE_LIMIT,
            max_parts: DEFAULT_MAX_PARTS,
            error_handler: None,
        }
    }
}

#[derive(Error, Debug)]
pub enum MultipartFormError {
    #[error("Multipart error: {0}")]
    Multipart(#[source] MultipartError),
    #[error("Deserialization error: {0}")]
    Deserialization(
        #[from]
        #[source]
        Error,
    ),
}

impl ResponseError for MultipartFormError {
    fn status_code(&self) -> StatusCode {
        match &self {
            MultipartFormError::Multipart(m) => m.status_code(),
            MultipartFormError::Deserialization(d) => d.status_code(),
        }
    }
}
