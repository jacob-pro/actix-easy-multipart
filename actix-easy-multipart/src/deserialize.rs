//! Helper types used for extracting and deserializing a multipart into a struct.

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
    #[error("Text field '{field_name}' couldn't be parsed: {source}")]
    ParseError {
        field_name: String,
        source: Box<dyn std::error::Error + Send>,
    },
    #[error("Unexpected part found with name '{0}'")]
    UnexpectedPart(String),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct FromFieldConfig {
    pub deny_extra_parts: bool,
}

#[doc(hidden)]
pub trait FromField
where
    Self: std::marker::Sized,
{
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error>;
}

#[doc(hidden)]
pub trait FromFieldExt
where
    Self: std::marker::Sized,
{
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error>;
}

impl<T, E> FromField for T
where
    T: FromStr<Err = E>,
    E: std::error::Error + Send + 'static,
{
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let mut matches = Vec::<T>::from_fields(fields, config, field_name)?;
        match matches.len() {
            0 => Err(Error::TextNotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ if config.deny_extra_parts => Err(Error::UnexpectedPart(field_name.into())),
            _ => Ok(matches.pop().unwrap()),
        }
    }
}

impl<T, E> FromFieldExt for Option<T>
where
    T: FromStr<Err = E>,
    E: std::error::Error + Send + 'static,
{
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let mut matches = Vec::<T>::from_fields(fields, config, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ if config.deny_extra_parts => Err(Error::UnexpectedPart(field_name.into())),
            _ => Ok(Some(matches.pop().unwrap())),
        }
    }
}

impl<T, E> FromFieldExt for Vec<T>
where
    T: FromStr<Err = E>,
    E: std::error::Error + Send + 'static,
{
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let total = fields.len();
        let texts = fields
            .into_iter()
            .filter_map(Field::text)
            .map(|text| {
                T::from_str(&text.text).map_err(|source| Error::ParseError {
                    field_name: field_name.to_string(),
                    source: Box::new(source),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if config.deny_extra_parts && texts.len() < total {
            // Non texts found
            return Err(Error::UnexpectedPart(field_name.to_string()));
        }
        Ok(texts)
    }
}

impl FromField for File {
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let mut matches = Vec::<File>::from_fields(fields, config, field_name)?;
        match matches.len() {
            0 => Err(Error::FileNotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ if config.deny_extra_parts => Err(Error::UnexpectedPart(field_name.into())),
            _ => Ok(matches.pop().unwrap()),
        }
    }
}

impl FromFieldExt for Option<File> {
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let mut matches = Vec::<File>::from_fields(fields, config, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ if config.deny_extra_parts => Err(Error::UnexpectedPart(field_name.into())),
            _ => Ok(Some(matches.pop().unwrap())),
        }
    }
}

impl FromFieldExt for Vec<File> {
    fn from_fields(
        fields: Vec<Field>,
        config: &FromFieldConfig,
        field_name: &str,
    ) -> Result<Self, Error> {
        let total = fields.len();
        let files = fields
            .into_iter()
            .filter_map(Field::file)
            .collect::<Vec<_>>();
        if config.deny_extra_parts && files.len() < total {
            // Non files found
            return Err(Error::UnexpectedPart(field_name.to_string()));
        }
        Ok(files)
    }
}
