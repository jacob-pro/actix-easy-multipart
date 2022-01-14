//! Validated multipart data extractor.

use super::{load_parts, Multiparts};
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
use validator::{Validate, ValidationErrors};

/// Validated multipart data extractor (`multipart/form-data`).
///
/// Can be used to extract multipart data from the request body, and automatically validate it.
///
/// [MultipartFormConfig] allows you to configure extraction process.
///
/// # Example
/// First define a structure to represent the form that implements `FromMultipart` and
/// [Validate] traits. Then use the extractor in your route.
///
/// ```
/// # #[macro_use] extern crate validator_derive;
/// # fn main() {
/// # use actix_easy_multipart_derive::FromMultipart;
/// # use validator::Validate;
/// #[derive(FromMultipart, Validate)]
/// struct Upload {
///    #[validate(length(max = 4096))]
///    description: String,
///    image: MultipartFile,
/// }
/// # use actix_web::Responder;
/// # use actix_easy_multipart::MultipartFile;
/// # use actix_easy_multipart::validated::MultipartForm;
///
/// async fn route(form: MultipartForm<Upload>) -> impl Responder {
///     format!("Received image of size: {}", form.image.size)
/// }
/// # }
/// ```
pub struct MultipartForm<T: Validate>(pub T);

impl<T: Validate> MultipartForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Validate> ops::Deref for MultipartForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Validate> ops::DerefMut for MultipartForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> FromRequest for MultipartForm<T>
where
    T: TryFrom<Multiparts, Error = Error> + Validate + 'static,
{
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = MultipartFormConfig;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = req.app_data::<Self::Config>().cloned().unwrap_or_default();

        let mp = Multipart::new(req.headers(), payload.take());
        load_parts(mp, config.text_limit, config.file_limit, config.max_parts)
            .map(move |res| match res {
                Ok(item) => {
                    let form = T::try_from(item)?;
                    form.validate()?;
                    Ok(form)
                }
                Err(e) => Err(MultipartFormError::Multipart(e)),
            })
            .map_ok(MultipartForm)
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
/// # use actix_easy_multipart::validated;
/// scope("/").app_data(
///     validated::MultipartFormConfig::default().file_limit(25 * 1024 * 1024) // 25 MiB
/// );
/// ```
#[derive(Clone)]
pub struct MultipartFormConfig {
    text_limit: usize,
    file_limit: u64,
    max_parts: usize,
    error_handler: Option<Rc<dyn Fn(MultipartFormError, &HttpRequest) -> actix_web::Error>>,
}

impl MultipartFormConfig {
    /// Change max number bytes of text in the multipart. Defaults to [DEFAULT_TEXT_LIMIT].
    pub fn text_limit(mut self, limit: usize) -> Self {
        self.text_limit = limit;
        self
    }

    /// Change max number of bytes for all files in the multipart.
    /// Defaults to [DEFAULT_FILE_LIMIT].
    pub fn file_limit(mut self, limit: u64) -> Self {
        self.file_limit = limit;
        self
    }

    /// Change max number of parts in the form. Defaults to [DEFAULT_MAX_PARTS].
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
    Multipart(MultipartError),
    #[error("Deserialization error: {0}")]
    Deserialization(
        #[from]
        #[source]
        Error,
    ),
    #[error("Validation error: {0}")]
    Validation(
        #[from]
        #[source]
        ValidationErrors,
    ),
}

impl ResponseError for MultipartFormError {
    fn status_code(&self) -> StatusCode {
        match &self {
            MultipartFormError::Multipart(m) => m.status_code(),
            MultipartFormError::Deserialization(d) => d.status_code(),
            MultipartFormError::Validation(..) => StatusCode::BAD_REQUEST,
        }
    }
}
