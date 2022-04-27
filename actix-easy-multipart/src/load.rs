//! Utility for loading a multipart form from an Actix multipart request.

use crate::{Field, File, Text, DEFAULT_FILE_LIMIT, DEFAULT_MAX_PARTS, DEFAULT_TEXT_LIMIT};
use actix_multipart::MultipartError;
use actix_web::error::{ParseError, PayloadError};
use actix_web::http::header;
use actix_web::http::header::DispositionType;
use actix_web::web::BytesMut;
use futures::{StreamExt, TryStreamExt};
use multimap::MultiMap;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

/// A multipart form of fields grouped by name.
pub type GroupedFields = MultiMap<String, Field>;

/// Utility for loading a multipart form from an [Actix Multipart](actix_multipart::Multipart)
/// request.
///
/// # Example
/// ```
/// # use actix_easy_multipart::load::Loader;
/// # use actix_web::{HttpResponse, Error};
/// async fn route(payload: actix_multipart::Multipart) -> Result<HttpResponse, Error> {
///     let parts = Loader::default().load_fields(payload).await?;
///     # unimplemented!()
/// }
/// ```
#[derive(Clone)]
pub struct Loader {
    text_limit: usize,
    file_limit: usize,
    max_parts: usize,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            text_limit: DEFAULT_TEXT_LIMIT,
            file_limit: DEFAULT_FILE_LIMIT,
            max_parts: DEFAULT_MAX_PARTS,
        }
    }
}

impl Loader {
    /// Use to configure a [Loader].
    pub fn builder() -> Builder {
        Builder {
            text_limit: DEFAULT_TEXT_LIMIT,
            file_limit: DEFAULT_FILE_LIMIT,
            max_parts: DEFAULT_MAX_PARTS,
        }
    }

    /// Load fields from a [Multipart](actix_multipart::Multipart) request into memory and disk.
    ///
    /// returns: A [Vec] of the parts in the order they were received.
    pub async fn load_fields(
        self,
        mut payload: actix_multipart::Multipart,
    ) -> Result<Vec<Field>, MultipartError> {
        // Implementation Notes:
        // https://tools.ietf.org/html/rfc7578#section-1
        // `content-type` defaults to text/plain
        // files SHOULD use appropriate mime or application/octet-stream
        // `filename` SHOULD be included but is not a MUST

        let mut parts = Vec::new();
        let mut text_budget = self.text_limit;
        let mut file_budget = self.file_limit;

        while let Ok(Some(field)) = payload.try_next().await {
            if parts.len() >= self.max_parts {
                return Err(MultipartError::Payload(PayloadError::Overflow));
            }
            let cd = field.content_disposition();
            match cd.disposition {
                DispositionType::FormData => {}
                _ => return Err(MultipartError::Parse(ParseError::Header)),
            }
            let name = match cd.get_name() {
                Some(name) => name.to_owned(),
                None => return Err(MultipartError::Parse(ParseError::Header)),
            };

            // We need to default to TEXT_PLAIN however actix content_type() defaults to APPLICATION_OCTET_STREAM
            let content_type = if field.headers().get(&header::CONTENT_TYPE).is_none() {
                mime::TEXT_PLAIN
            } else {
                field.content_type().clone()
            };

            let item = if content_type == mime::TEXT_PLAIN && cd.get_filename().is_none() {
                let (r, size) = create_text(field, name, text_budget).await?;
                text_budget -= size;
                Field::Text(r)
            } else {
                let filename = cd.get_filename().map(|f| f.to_owned());
                let r = create_file(field, name, filename, file_budget, content_type).await?;
                file_budget -= r.size;
                Field::File(r)
            };
            parts.push(item);
        }
        Ok(parts)
    }

    /// Load fields from a [Multipart](actix_multipart::Multipart) request into memory and disk.
    ///
    /// returns: A MultiMap grouping fields by their name.
    pub async fn load_grouped(
        self,
        payload: actix_multipart::Multipart,
    ) -> Result<GroupedFields, MultipartError> {
        let parts = self.load_fields(payload).await?;
        Ok(parts
            .into_iter()
            .map(|part| (part.name().to_owned(), part))
            .collect())
    }
}

async fn create_file(
    mut field: actix_multipart::Field,
    name: String,
    filename: Option<String>,
    max_size: usize,
    mime: mime::Mime,
) -> Result<File, MultipartError> {
    let mut written = 0;
    let mut budget = max_size;
    let ntf = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => return Err(MultipartError::Payload(PayloadError::Io(e))),
    };
    let mut async_file = tokio::fs::File::from_std(
        ntf.reopen()
            .map_err(|e| MultipartError::Payload(PayloadError::Io(e)))?,
    );
    while let Some(chunk) = field.next().await {
        let bytes = chunk?;
        let length = bytes.len();
        if budget < length {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
        async_file
            .write_all(bytes.as_ref())
            .await
            .map_err(|e| MultipartError::Payload(PayloadError::Io(e)))?;
        written += length;
        budget -= length;
    }
    async_file
        .flush()
        .await
        .map_err(|e| MultipartError::Payload(PayloadError::Io(e)))?;
    Ok(File {
        file: ntf,
        size: written,
        name,
        filename,
        mime,
    })
}

async fn create_text(
    mut field: actix_multipart::Field,
    name: String,
    max_length: usize,
) -> Result<(Text, usize), MultipartError> {
    let mut written = 0;
    let mut budget = max_length;
    let mut acc = BytesMut::new();

    while let Some(chunk) = field.next().await {
        let bytes = chunk?;
        let length = bytes.len();
        if budget < length {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
        acc.extend(bytes);
        written += length;
        budget -= length;
    }

    let text = String::from_utf8(acc.to_vec())
        .map_err(|a| MultipartError::Parse(ParseError::Utf8(a.utf8_error())))?;
    Ok((Text { name, text }, written))
}

/// Allows configuring the [Loader] parameters.
pub struct Builder {
    text_limit: usize,
    file_limit: usize,
    max_parts: usize,
}

impl Builder {
    pub fn build(&self) -> Loader {
        Loader {
            text_limit: self.text_limit,
            file_limit: self.file_limit,
            max_parts: self.max_parts,
        }
    }

    /// Set maximum allowed bytes of text in the form.
    pub fn text_limit(mut self, text_limit: usize) -> Self {
        self.text_limit = text_limit;
        self
    }

    /// Set maximum allowed bytes for all files in the form.
    pub fn file_limit(mut self, file_limit: usize) -> Self {
        self.file_limit = file_limit;
        self
    }

    /// Set maximum allowed parts in the form.
    pub fn max_parts(mut self, max_parts: usize) -> Self {
        self.max_parts = max_parts;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{web, App, Error, HttpResponse};
    use serde::{Deserialize, Serialize};
    use std::io::Write;
    use tokio::io::AsyncReadExt;

    #[derive(Serialize, Deserialize, Debug)]
    struct Response {
        string: String,
        file_content: String,
    }

    async fn test_route(payload: actix_multipart::Multipart) -> Result<HttpResponse, Error> {
        let parts = Loader::default().load_grouped(payload).await?;

        let mut file_content = String::new();
        let file = parts.get("file").unwrap().file_ref().unwrap();
        let mut file = tokio::fs::File::from_std(file.file.reopen().unwrap());
        file.read_to_string(&mut file_content).await.unwrap();

        let string = parts
            .get("string")
            .unwrap()
            .text_ref()
            .unwrap()
            .text
            .clone();

        Ok(HttpResponse::Ok().json(Response {
            string,
            file_content,
        }))
    }

    #[actix_rt::test]
    async fn test() {
        let srv = actix_test::start(|| App::new().route("/", web::post().to(test_route)));

        let temp = NamedTempFile::new().unwrap();
        temp.as_file()
            .write_all("File contents".as_bytes())
            .unwrap();
        let tokio_handle = tokio::fs::File::from_std(temp.reopen().unwrap());

        let form = reqwest::multipart::Form::new()
            .text("string", "Hello World")
            .part(
                "file",
                reqwest::multipart::Part::stream(tokio_handle).file_name("name"),
            );

        let response = reqwest::Client::default()
            .post(srv.url("/"))
            .multipart(form)
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());
        let res: Response = response.json().await.unwrap();
        assert_eq!(res.string, "Hello World");
        assert_eq!(res.file_content, "File contents");
    }

    async fn file_size_limit_route(
        payload: actix_multipart::Multipart,
    ) -> Result<HttpResponse, Error> {
        Loader::builder()
            .file_limit(2)
            .build()
            .load_fields(payload)
            .await?;
        Ok(HttpResponse::Ok().into())
    }

    #[actix_rt::test]
    async fn file_size_limit_test() {
        let srv =
            actix_test::start(|| App::new().route("/", web::post().to(file_size_limit_route)));

        let temp = NamedTempFile::new().unwrap();
        temp.as_file()
            .write_all("More than two bytes!!!".as_bytes())
            .unwrap();
        let tokio_handle = tokio::fs::File::from_std(temp.reopen().unwrap());

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::stream(tokio_handle).file_name("name"),
        );

        let response = reqwest::Client::default()
            .post(srv.url("/"))
            .multipart(form)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            "Payload reached size limit.",
            response.text().await.unwrap()
        );
    }
}
