//! Deserializes a field from plain text.
use crate::bytes::Bytes;
use crate::{field_mime, FieldReader, Limits};
use actix_multipart::Field;
use actix_web::http::StatusCode;
use actix_web::{web, Error, HttpRequest, ResponseError};
use derive_more::{Deref, DerefMut, Display, Error};
use futures_core::future::LocalBoxFuture;
use futures_util::FutureExt;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Deserialize from plain text.
///
/// Internally this uses [`serde_plain`] for deserialization, which supports primitive types
/// including strings, numbers, and simple enums.
#[derive(Debug, Deref, DerefMut)]
pub struct Text<T: DeserializeOwned>(pub T);

impl<T: DeserializeOwned> Text<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'t, T: DeserializeOwned + 'static> FieldReader<'t> for Text<T> {
    type Future = LocalBoxFuture<'t, Result<Self, crate::Error>>;

    fn read_field(req: &'t HttpRequest, field: Field, limits: &'t mut Limits) -> Self::Future {
        async move {
            let config = TextConfig::from_req(req);
            let field_name = field.name().to_owned();

            if config.validate_content_type {
                let valid = if let Some(mime) = field_mime(&field) {
                    mime.subtype() == mime::PLAIN || mime.suffix() == Some(mime::PLAIN)
                } else {
                    // https://www.rfc-editor.org/rfc/rfc7578#section-4.4
                    // content type defaults to text/plain, so None should be considered valid
                    true
                };
                if !valid && config.validate_content_type {
                    return Err(crate::Error::Field {
                        field_name,
                        source: config.map_error(req, TextError::ContentType),
                    });
                }
            }

            let bytes = Bytes::read_field(req, field, limits).await?;

            let text =
                std::str::from_utf8(bytes.data.as_ref()).map_err(|e| crate::Error::Field {
                    field_name: field_name.clone(),
                    source: config.map_error(req, TextError::Utf8Error(e)),
                })?;

            Ok(Text(serde_plain::from_str(text).map_err(|e| {
                crate::Error::Field {
                    field_name,
                    source: config.map_error(req, TextError::Deserialize(e)),
                }
            })?))
        }
        .boxed_local()
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum TextError {
    /// Utf8 error
    #[display(fmt = "Utf8 decoding error: {}", _0)]
    Utf8Error(std::str::Utf8Error),

    /// Deserialize error
    #[display(fmt = "Plain text deserialize error: {}", _0)]
    Deserialize(serde_plain::Error),

    /// Content type error
    #[display(fmt = "Content type error")]
    ContentType,
}

impl ResponseError for TextError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Configuration for the [`Text`] field reader.
#[derive(Clone)]
pub struct TextConfig {
    err_handler: Option<Arc<dyn Fn(TextError, &HttpRequest) -> Error + Send + Sync>>,
    validate_content_type: bool,
}

const DEFAULT_CONFIG: TextConfig = TextConfig {
    err_handler: None,
    validate_content_type: true,
};

impl TextConfig {
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(TextError, &HttpRequest) -> Error + Send + Sync + 'static,
    {
        self.err_handler = Some(Arc::new(f));
        self
    }

    /// Extract payload config from app data. Check both `T` and `Data<T>`, in that order, and fall
    /// back to the default payload config.
    fn from_req(req: &HttpRequest) -> &Self {
        req.app_data::<Self>()
            .or_else(|| req.app_data::<web::Data<Self>>().map(|d| d.as_ref()))
            .unwrap_or(&DEFAULT_CONFIG)
    }

    fn map_error(&self, req: &HttpRequest, err: TextError) -> Error {
        if let Some(err_handler) = self.err_handler.as_ref() {
            (*err_handler)(err, req)
        } else {
            err.into()
        }
    }

    /// Sets whether or not the field must have a valid `Content-Type` header to be parsed.
    /// Note that an empty `Content-Type` is also accepted, as the multipart specification defines
    /// `text/plain` as the default for text fields.
    pub fn validate_content_type(mut self, validate_content_type: bool) -> Self {
        self.validate_content_type = validate_content_type;
        self
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        DEFAULT_CONFIG
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::send_form;
    use crate::text::{Text, TextConfig};
    use crate::MultipartForm;
    use actix_multipart_rfc7578::client::multipart;
    use actix_web::http::StatusCode;
    use actix_web::{web, App, HttpResponse, Responder};
    use std::io::Cursor;

    #[derive(MultipartForm)]
    struct TextForm {
        number: Text<i32>,
    }

    async fn test_text_route(form: MultipartForm<TextForm>) -> impl Responder {
        assert_eq!(*form.number, 1025);
        HttpResponse::Ok().finish()
    }

    #[actix_rt::test]
    async fn test_content_type_validation() {
        let srv = actix_test::start(|| {
            App::new()
                .route("/", web::post().to(test_text_route))
                .app_data(TextConfig::default().validate_content_type(true))
        });

        // Deny because wrong content type
        let bytes = Cursor::new("1025");
        let mut form = multipart::Form::default();
        form.add_reader_file_with_mime("number", bytes, "", mime::APPLICATION_OCTET_STREAM);
        let response = send_form(&srv, form, "/").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Allow because correct content type
        let bytes = Cursor::new("1025");
        let mut form = multipart::Form::default();
        form.add_reader_file_with_mime("number", bytes, "", mime::TEXT_PLAIN);
        let response = send_form(&srv, form, "/").await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
