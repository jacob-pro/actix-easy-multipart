# Actix Easy Multipart

[![Build status](https://github.com/jacob-pro/actix-easy-multipart/actions/workflows/rust.yml/badge.svg)](https://github.com/jacob-pro/actix-easy-multipart/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/actix-easy-multipart.svg)](https://crates.io/crates/actix-easy-multipart)
[![docs.rs](https://docs.rs/actix-easy-multipart/badge.svg)](https://docs.rs/actix-easy-multipart/latest/actix_easy_multipart/)

Typed multipart form extractor for [actix-web](https://github.com/actix/actix-web).

## Example

```rust
use actix_web::Responder;
use actix_easy_multipart::tempfile::Tempfile;
use actix_easy_multipart::text::Text;
use actix_easy_multipart::MultipartForm;

#[derive(MultipartForm)]
struct Upload {
    description: Option<Text<String>>,
    timestamp: Text<i64>,
    #[multipart(rename="image_set[]")]
    image_set: Vec<Tempfile>,
}

async fn route(form: MultipartForm<Upload>) -> impl Responder {
    format!("Received 5 images: {}", form.image_set.len())
}
```

## Features

- Receiving optional fields, using `Option`.
- Receiving [lists of fields](https://www.rfc-editor.org/rfc/rfc7578#section-4.3), using `Vec<T>`.
- Deserialize integers, floats, enums from plain text fields using `Text<T>`.
- Deserialize complex data from JSON uploads, using `Json<T>`.
- Receive file uploads into temporary files on disk, using `Tempfile`.
- User customisable asynchronous field readers, for example you may want to stream form data to an object storage 
  service, just implement the `FieldReader` trait.

## Versions and Compatibility

| actix-easy-multipart                                              | actix-web | tokio |
|-------------------------------------------------------------------|-----------|-------|
| [0.x](https://github.com/jacob-pro/actix-easy-multipart/tree/0.x) | 2.x       | 0.2   |
| [1.x](https://github.com/jacob-pro/actix-easy-multipart/tree/1.x) | 3.x       | 0.2   |
| 2.x                                                               | 4.x       | 1     |
| 3.x                                                               | 4.x       | 1     |

## See Also

- [Pull request to add this to actix-multipart](https://github.com/actix/actix-web/pull/2883)
- [Discussion thread in actix-web](https://github.com/actix/actix-web/issues/2849)
