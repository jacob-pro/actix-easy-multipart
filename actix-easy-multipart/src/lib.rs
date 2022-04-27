//! Provides an easy to use HTTP multipart form extractor for the
//! [actix-web](https://github.com/actix/actix-web) framework.
//!
//! The extractor writes temporary files on disk using the
//! [tempfile](https://github.com/Stebalien/tempfile) crate with very similar behaviour to the
//! [$_FILES variable in PHP](https://www.php.net/manual/en/reserved.variables.files.php#89674).

#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export derive
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate actix_easy_multipart_derive;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use actix_easy_multipart_derive::FromMultipart;

pub mod deserialize;
pub mod extractor;
pub mod load;
#[cfg(feature = "validator")]
pub mod validated;

use deserialize::Error;
use std::ffi::OsStr;
use std::path::Path;
use tempfile::NamedTempFile;

const DEFAULT_TEXT_LIMIT: usize = 16384; // 16 KiB
const DEFAULT_FILE_LIMIT: usize = 51200; // 50 MiB
const DEFAULT_MAX_PARTS: usize = 1000;

/// A Field in a multipart form.
#[derive(Debug)]
pub enum Field {
    File(File),
    Text(Text),
}

impl Field {
    pub fn name(&self) -> &str {
        match &self {
            Field::File(f) => &f.name,
            Field::Text(t) => &t.name,
        }
    }

    pub fn text(&self) -> Option<&Text> {
        match self {
            Field::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn file(&self) -> Option<&File> {
        match self {
            Field::File(f) => Some(f),
            _ => None,
        }
    }
}

/// An uploaded file in a multipart form.
///
/// A part is treated as a file upload if the `Content-Type` header is set to anything
/// other than `text/plain` or if a `filename` is specified in the `Content-Disposition` header.
#[derive(Debug)]
pub struct File {
    /// The file data itself stored as a temporary file on disk.
    pub file: NamedTempFile,
    /// The size in bytes of the file.
    pub size: usize,
    /// The name of the field in the multipart form.
    pub name: String,
    /// The `filename` value in the `Content-Disposition` header.
    pub filename: Option<String>,
    /// The Content-Type specified as reported in the uploaded form.
    /// # Security
    /// This is provided by the client so should not be trusted.
    pub mime: mime::Mime,
}

impl File {
    /// Get the extension portion of the `filename` value in the `Content-Disposition` header.
    pub fn get_extension(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|f| Path::new(f.as_str()).extension().and_then(OsStr::to_str))
    }
}

/// A text field in a multipart form.
///
/// A body part is treated as text if the `Content-Type` header is either `None` or equal
/// to `text/plain`, and no `filename` is specified in the `Content-Disposition` header.
#[derive(Clone, Debug)]
pub struct Text {
    /// The name of the field in the multipart form.
    pub name: String,
    /// The text body of the field / part.
    pub text: String,
}
