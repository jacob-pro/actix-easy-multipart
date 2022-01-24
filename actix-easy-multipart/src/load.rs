use crate::{MultipartField, MultipartFile, MultipartText, Multiparts};
use actix_multipart::MultipartError;
use actix_web::error::{ParseError, PayloadError};
use actix_web::http::header;
use actix_web::http::header::DispositionType;
use actix_web::web::BytesMut;
use futures::{StreamExt, TryStreamExt};
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub const DEFAULT_TEXT_LIMIT: usize = 1024 * 1024;
pub const DEFAULT_FILE_LIMIT: u64 = 512 * 1024 * 1024;
pub const DEFAULT_MAX_PARTS: usize = 1000;

// Implementation Notes:
// https://tools.ietf.org/html/rfc7578#section-1
// `content-type` defaults to text/plain
// files SHOULD use appropriate mime or application/octet-stream
// `filename` SHOULD be included but is not a MUST

/// Use to load an [actix_multipart::Multipart] request into [Multiparts].
///
/// **In general you should favour using the [MultipartForm](crate::extractor::MultipartForm)
/// extractor or its validated version [MultipartForm](crate::validated::MultipartForm).**
///
/// # Example
/// ```
/// # use actix_easy_multipart::{load_parts, DEFAULT_TEXT_LIMIT, DEFAULT_FILE_LIMIT, DEFAULT_MAX_PARTS};
/// # use actix_web::{HttpResponse, Error};
/// async fn route(payload: actix_multipart::Multipart) -> Result<HttpResponse, Error> {
///     let parts = load_parts(
///         payload,
///         DEFAULT_TEXT_LIMIT,
///         DEFAULT_FILE_LIMIT,
///         DEFAULT_MAX_PARTS,
///     )
///     .await?;
///     # unimplemented!()
/// }
/// ```
pub async fn load_parts(
    mut payload: actix_multipart::Multipart,
    text_limit: usize,
    file_limit: u64,
    max_parts: usize,
) -> Result<Multiparts, MultipartError> {
    let mut parts = Multiparts::new();
    let mut text_budget = text_limit;
    let mut file_budget = file_limit;

    while let Ok(Some(field)) = payload.try_next().await {
        if parts.len() >= max_parts {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
        let cd = match field.content_disposition() {
            Some(cd) => cd,
            None => return Err(MultipartError::Parse(ParseError::Header)),
        };
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
            MultipartField::Text(r)
        } else {
            let filename = cd.get_filename().map(|f| f.to_owned());
            let r = create_file(field, name, filename, file_budget, content_type).await?;
            file_budget -= r.size;
            MultipartField::File(r)
        };
        parts.push(item);
    }
    Ok(parts)
}

async fn create_file(
    mut field: actix_multipart::Field,
    name: String,
    filename: Option<String>,
    max_size: u64,
    mime: mime::Mime,
) -> Result<MultipartFile, MultipartError> {
    let mut written = 0;
    let mut budget = max_size;
    let ntf = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => return Err(MultipartError::Payload(PayloadError::Io(e))),
    };
    let mut async_file = File::from_std(
        ntf.reopen()
            .map_err(|e| MultipartError::Payload(PayloadError::Io(e)))?,
    );
    while let Some(chunk) = field.next().await {
        let bytes = chunk?;
        let length = bytes.len() as u64;
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
    Ok(MultipartFile {
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
) -> Result<(MultipartText, usize), MultipartError> {
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
    //TODO: Currently only supports UTF-8, consider looking at the charset header and _charset_ field
    let text = String::from_utf8(acc.to_vec())
        .map_err(|a| MultipartError::Parse(ParseError::Utf8(a.utf8_error())))?;
    Ok((MultipartText { name, text }, written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deserialize::RetrieveFromMultiparts;
    use actix_multipart::Multipart;
    use actix_multipart_rfc7578::client::multipart;
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App, Error, HttpResponse};
    use awc::Client;
    use serde::{Deserialize, Serialize};
    use std::io::{Read, Write};

    #[derive(Serialize, Deserialize, Debug)]
    struct Response {
        string: String,
        int: i32,
        file_content: String,
    }

    async fn test_route(payload: Multipart) -> Result<HttpResponse, Error> {
        let mut k = load_parts(
            payload,
            DEFAULT_TEXT_LIMIT,
            DEFAULT_FILE_LIMIT,
            DEFAULT_MAX_PARTS,
        )
        .await?;

        let mut data = String::new();
        let f: MultipartFile = RetrieveFromMultiparts::get_from_multiparts(&mut k, "file")?;
        f.file.into_file().read_to_string(&mut data).unwrap();

        let r = Response {
            string: RetrieveFromMultiparts::get_from_multiparts(&mut k, "string")?,
            int: RetrieveFromMultiparts::get_from_multiparts(&mut k, "int")?,
            file_content: data,
        };
        Ok(HttpResponse::Ok().json(r))
    }

    #[actix_rt::test]
    async fn test() {
        let srv = test::start(|| App::new().route("/", web::post().to(test_route)));

        let mut form = multipart::Form::default();
        form.add_text("string", "Hello World");
        form.add_text("int", "69");

        let temp = NamedTempFile::new().unwrap();
        temp.as_file()
            .write_all("File contents".as_bytes())
            .unwrap();
        form.add_file("file", temp.path()).unwrap();

        let mut response = Client::default()
            .post(srv.url("/"))
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();

        assert!(response.status().is_success());
        let res: Response = response.json().await.unwrap();
        assert_eq!(res.string, "Hello World");
        assert_eq!(res.int, 69);
        assert_eq!(res.file_content, "File contents");
    }

    async fn file_size_limit_route(payload: Multipart) -> Result<HttpResponse, Error> {
        load_parts(payload, DEFAULT_TEXT_LIMIT, 2, DEFAULT_MAX_PARTS).await?;
        Ok(HttpResponse::Ok().into())
    }

    #[actix_rt::test]
    async fn file_size_limit_test() {
        let srv = test::start(|| App::new().route("/", web::post().to(file_size_limit_route)));

        let mut form = multipart::Form::default();
        let temp = NamedTempFile::new().unwrap();
        temp.as_file()
            .write_all("More than two bytes!!!".as_bytes())
            .unwrap();
        form.add_file("file", temp.path()).unwrap();

        let mut response = Client::default()
            .post(srv.url("/"))
            .content_type(form.content_type())
            .send_body(multipart::Body::from(form))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            "A payload reached size limit.",
            response.body().await.unwrap()
        );
    }
}
