//! Deserializes a field as JSON.
use crate::bytes::Bytes;
use crate::{field_mime, FieldReader, Limits};
use actix_multipart::Field;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, ResponseError};
use derive_more::{Deref, DerefMut, Display, Error};
use futures_core::future::LocalBoxFuture;
use futures_util::FutureExt;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Deserialize from JSON.
#[derive(Debug, Deref, DerefMut)]
pub struct Json<T: DeserializeOwned>(pub T);

impl<T: DeserializeOwned> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'t, T: DeserializeOwned + 'static> FieldReader<'t> for Json<T> {
    type Future = LocalBoxFuture<'t, Result<Self, crate::Error>>;

    fn read_field(req: &'t HttpRequest, field: Field, limits: &'t mut Limits) -> Self::Future {
        async move {
            let config = JsonConfig::from_req(req);
            let field_name = field.name().to_owned();

            if config.validate_content_type {
                let valid = if let Some(mime) = field_mime(&field) {
                    mime.subtype() == mime::JSON || mime.suffix() == Some(mime::JSON)
                } else {
                    false
                };
                if !valid {
                    return Err(crate::Error::Field {
                        field_name,
                        source: config.map_error(req, JsonFieldError::ContentType),
                    });
                }
            }

            let bytes = Bytes::read_field(req, field, limits).await?;

            Ok(Json(serde_json::from_slice(bytes.data.as_ref()).map_err(
                |e| crate::Error::Field {
                    field_name,
                    source: config.map_error(req, JsonFieldError::Deserialize(e)),
                },
            )?))
        }
        .boxed_local()
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum JsonFieldError {
    /// Deserialize error
    #[display(fmt = "Json deserialize error: {}", _0)]
    Deserialize(serde_json::Error),

    /// Content type error
    #[display(fmt = "Content type error")]
    ContentType,
}

impl ResponseError for JsonFieldError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Configuration for the [`Json`] field reader.
#[derive(Clone)]
pub struct JsonConfig {
    err_handler:
        Option<Arc<dyn Fn(JsonFieldError, &HttpRequest) -> actix_web::Error + Send + Sync>>,
    validate_content_type: bool,
}

const DEFAULT_CONFIG: JsonConfig = JsonConfig {
    err_handler: None,
    validate_content_type: true,
};

impl JsonConfig {
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(JsonFieldError, &HttpRequest) -> actix_web::Error + Send + Sync + 'static,
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

    fn map_error(&self, req: &HttpRequest, err: JsonFieldError) -> actix_web::Error {
        if let Some(err_handler) = self.err_handler.as_ref() {
            (*err_handler)(err, req)
        } else {
            err.into()
        }
    }

    /// Sets whether or not the field must have a valid `Content-Type` header to be parsed.
    pub fn validate_content_type(mut self, validate_content_type: bool) -> Self {
        self.validate_content_type = validate_content_type;
        self
    }
}

impl Default for JsonConfig {
    fn default() -> Self {
        DEFAULT_CONFIG
    }
}

#[cfg(test)]
mod tests {
    use crate::json::{Json, JsonConfig};
    use crate::tests::send_form;
    use crate::MultipartForm;
    use actix_multipart_rfc7578::client::multipart;
    use actix_web::http::StatusCode;
    use actix_web::{web, App, HttpResponse, Responder};
    use std::collections::HashMap;
    use std::io::Cursor;

    #[derive(MultipartForm)]
    struct JsonForm {
        json: Json<HashMap<String, String>>,
    }

    async fn test_json_route(form: MultipartForm<JsonForm>) -> impl Responder {
        let mut expected = HashMap::new();
        expected.insert("key1".to_owned(), "value1".to_owned());
        expected.insert("key2".to_owned(), "value2".to_owned());
        assert_eq!(&*form.json, &expected);
        HttpResponse::Ok().finish()
    }

    #[actix_rt::test]
    async fn test_json_without_content_type() {
        let srv = actix_test::start(|| {
            App::new()
                .route("/", web::post().to(test_json_route))
                .app_data(JsonConfig::default().validate_content_type(false))
        });

        let mut form = multipart::Form::default();
        form.add_text("json", "{\"key1\": \"value1\", \"key2\": \"value2\"}");
        let response = send_form(&srv, form, "/").await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_content_type_validation() {
        let srv = actix_test::start(|| {
            App::new()
                .route("/", web::post().to(test_json_route))
                .app_data(JsonConfig::default().validate_content_type(true))
        });

        // Deny because wrong content type
        let bytes = Cursor::new("{\"key1\": \"value1\", \"key2\": \"value2\"}");
        let mut form = multipart::Form::default();
        form.add_reader_file_with_mime("json", bytes, "", mime::APPLICATION_OCTET_STREAM);
        let response = send_form(&srv, form, "/").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Allow because correct content type
        let bytes = Cursor::new("{\"key1\": \"value1\", \"key2\": \"value2\"}");
        let mut form = multipart::Form::default();
        form.add_reader_file_with_mime("json", bytes, "", mime::APPLICATION_JSON);
        let response = send_form(&srv, form, "/").await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
