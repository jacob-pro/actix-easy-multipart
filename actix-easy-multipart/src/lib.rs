//! Provides an easy to use HTTP multipart form extractor for the
//! [actix-web](https://github.com/actix/actix-web) framework.
//!
//! The extractor writes temporary files on disk using the
//! [tempfile](https://github.com/Stebalien/tempfile) crate with similar behaviour to the
//! [$_FILES variable in PHP](https://www.php.net/manual/en/reserved.variables.files.php#89674).

// Re-export derive
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate actix_easy_multipart_derive;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use actix_easy_multipart_derive::FromMultipart;

pub mod deserialize;
pub mod extractor;
mod load;
#[cfg(feature = "validator")]
pub mod validated;

pub use load::{load_parts, DEFAULT_FILE_LIMIT, DEFAULT_MAX_PARTS, DEFAULT_TEXT_LIMIT};

use deserialize::Error;
use std::ffi::OsStr;
use std::path::Path;
use tempfile::NamedTempFile;

/// A list of [MultipartFields](MultipartField).
///
/// The [RetrieveFromMultiparts](deserialize::RetrieveFromMultiparts)
/// and [RetrieveFromMultipartsExt](deserialize::RetrieveFromMultipartsExt) traits
/// can be used to retrieving and parse a field by name.
pub type Multiparts = Vec<MultipartField>;

#[derive(Debug)]
pub enum MultipartField {
    File(MultipartFile),
    Text(MultipartText),
}

/// An uploaded file in a multipart form.
///
/// A part is treated as a file upload if the `Content-Type` header is set to anything
/// other than `text/plain` or if a `filename` is specified in the `Content-Disposition` header.
#[derive(Debug)]
pub struct MultipartFile {
    /// The file data itself stored as a temporary file on disk.
    pub file: NamedTempFile,
    /// The size in bytes of the file.
    pub size: u64,
    /// The name of the field in the multipart form.
    pub name: String,
    /// The `filename` value in the `Content-Disposition` header.
    pub filename: Option<String>,
    /// The Content-Type specified as reported in the uploaded form.
    /// # Security
    /// This is provided by the client so should not be trusted.
    pub mime: mime::Mime,
}

impl MultipartFile {
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
#[derive(Debug)]
pub struct MultipartText {
    /// The name of the field in the multipart form.
    pub name: String,
    /// The text body of the field / part.
    pub text: String,
}
