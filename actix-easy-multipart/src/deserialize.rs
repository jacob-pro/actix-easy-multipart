//! Traits for retrieving and parsing fields out of a multipart form.

use crate::{Field, File};
use actix_web::http::StatusCode;
use actix_web::ResponseError;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Text field '{0}' not found")]
    TextNotFound(String),
    #[error("File upload '{0}' not found")]
    FileNotFound(String),
    #[error("Text field '{field_name}' couldn't be parsed: {error}")]
    ParseError { field_name: String, error: String },
    #[error("Duplicate values found for '{0}'")]
    DuplicateField(String),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Allows retrieving a specific named field/part from a Multipart form.
pub trait RetrieveFromMultiparts
where
    Self: std::marker::Sized,
{
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error>;
}

/// Identical to [RetrieveFromMultiparts] but implemented for [Option] and [Vec].
///
/// This second trait is expected to not be needed once trait
/// [specialization](https://rust-lang.github.io/rfcs/1210-impl-specialization.html) is stable.
pub trait RetrieveFromMultipartsExt
where
    Self: std::marker::Sized,
{
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error>;
}

impl<T, E> RetrieveFromMultiparts for T
where
    T: FromStr<Err = E>,
    E: ToString,
{
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut matches = Vec::<T>::get_from_multiparts(form, field_name)?;
        match matches.len() {
            0 => Err(Error::TextNotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(Error::DuplicateField(field_name.into())),
        }
    }
}

impl<T, E> RetrieveFromMultipartsExt for Option<T>
where
    T: FromStr<Err = E>,
    E: ToString,
{
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut matches = Vec::<T>::get_from_multiparts(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(Error::DuplicateField(field_name.into())),
        }
    }
}

impl<T, E> RetrieveFromMultipartsExt for Vec<T>
where
    T: FromStr<Err = E>,
    E: ToString,
{
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut matches = Vec::new();
        for i in form {
            match i {
                Field::File(_) => {}
                Field::Text(x) => {
                    if x.name == field_name {
                        let y: T = x.text.parse().map_err(|e: E| Error::ParseError {
                            field_name: field_name.into(),
                            error: e.to_string(),
                        })?;
                        matches.push(y);
                    }
                }
            }
        }
        Ok(matches)
    }
}

impl RetrieveFromMultiparts for File {
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut matches = Vec::<File>::get_from_multiparts(form, field_name)?;
        match matches.len() {
            0 => Err(Error::FileNotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(Error::DuplicateField(field_name.into())),
        }
    }
}

impl RetrieveFromMultipartsExt for Option<File> {
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut matches = Vec::<File>::get_from_multiparts(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(Error::DuplicateField(field_name.into())),
        }
    }
}

impl RetrieveFromMultipartsExt for Vec<File> {
    fn get_from_multiparts(form: &mut Vec<Field>, field_name: &str) -> Result<Self, Error> {
        let mut indexes = Vec::new();
        for (idx, item) in form.iter().enumerate() {
            match item {
                Field::Text(_) => {}
                Field::File(x) => {
                    if x.name == field_name {
                        indexes.push(idx)
                    }
                }
            }
        }
        Ok(indexes
            .iter()
            .rev()
            .map(|idx| match form.remove(*idx) {
                Field::File(x) => x,
                Field::Text(_) => panic!(),
            })
            .collect())
    }
}
