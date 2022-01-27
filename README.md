# Actix Easy Multipart

[![Build status](https://github.com/jacob-pro/actix-easy-multipart/actions/workflows/rust.yml/badge.svg)](https://github.com/jacob-pro/actix-easy-multipart/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/actix-easy-multipart.svg)](https://crates.io/crates/actix-easy-multipart)
[![docs.rs](https://docs.rs/actix-easy-multipart/badge.svg)](https://docs.rs/actix-easy-multipart/latest/actix_easy_multipart/)

Easy to use Multipart Forms for [actix-web](https://github.com/actix/actix-web).

File uploads are written to disk as [temporary files](https://github.com/Stebalien/tempfile) similar to the way the
[$_FILES](https://www.php.net/manual/en/reserved.variables.files.php#89674) variable works in PHP.

## Example

```rust
use actix_web::Responder;
use actix_easy_multipart::{MultipartFile, FromMultipart};
use actix_easy_multipart::extractor::MultipartForm;

#[derive(FromMultipart)]
struct Upload {
   description: String,
   image: MultipartFile,
}

async fn route(form: MultipartForm<Upload>) -> impl Responder {
    format!("Received image of size: {}", form.image.size)
}
```

## Versions and Compatibility

| actix-easy-multipart                                              | actix-web | tokio |
|-------------------------------------------------------------------|-----------|-------|
| [0.x](https://github.com/jacob-pro/actix-easy-multipart/tree/0.x) | 2.x       | 0.2   |
| [1.x](https://github.com/jacob-pro/actix-easy-multipart/tree/1.x) | 3.x       | 0.2   |
| 2.x                                                               | 4.x       | 1     |
