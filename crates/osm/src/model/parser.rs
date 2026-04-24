#[derive(Debug, thiserror::Error)]
pub enum RecordParseError {
    #[error("missing field `{field}`")]
    MissingField { field: &'static str },

    #[error("invalid field `{field}`: {value}")]
    InvalidField { field: &'static str, value: String },

    #[error("unexpected type for `{field}`: {value}")]
    UnexpectedType { field: &'static str, value: String },

    #[error("unexpected shape type: {shape_type}")]
    UnexpectedShapeType { shape_type: ShapeType },

    #[error("invalid shape: {message}")]
    InvalidShape { message: String },

    #[error("shapefile error")]
    ShapefileError(#[from] shapefile::Error),
}

pub trait Parser {
    type Output;

    fn parse(
        field: &'static str,
        value: &shapefile::dbase::FieldValue,
    ) -> Result<Self::Output, RecordParseError>;
}

pub trait RecordExt {
    fn required<P: Parser>(&self, field: &'static str) -> Result<P::Output, RecordParseError>;

    fn optional<P: Parser>(
        &self,
        field: &'static str,
    ) -> Result<Option<P::Output>, RecordParseError>;
}

impl RecordExt for shapefile::dbase::Record {
    fn required<P: Parser>(&self, field: &'static str) -> Result<P::Output, RecordParseError> {
        let value = self
            .get(field)
            .ok_or(RecordParseError::MissingField { field })?;

        P::parse(field, value)
    }

    fn optional<P: Parser>(
        &self,
        field: &'static str,
    ) -> Result<Option<P::Output>, RecordParseError> {
        match self.get(field) {
            Some(v) => Ok(OptionParser::<P>::parse(field, v)?),
            None => Ok(None),
        }
    }
}

pub struct StringParser;

impl Parser for StringParser {
    type Output = String;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            FieldValue::Character(Some(v)) => Ok(v.trim().to_owned()),
            other => Err(RecordParseError::UnexpectedType {
                field,
                value: format!("{other:?}"),
            }),
        }
    }
}

pub struct I32Parser;

impl Parser for I32Parser {
    type Output = i32;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            FieldValue::Numeric(Some(v)) => Ok(*v as i32),
            other => Err(RecordParseError::UnexpectedType {
                field,
                value: format!("{other:?}"),
            }),
        }
    }
}

pub struct U32Parser;

impl Parser for U32Parser {
    type Output = u32;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            FieldValue::Numeric(Some(v)) if *v >= 0.0 => Ok(*v as u32),
            FieldValue::Numeric(Some(v)) => Err(RecordParseError::InvalidField {
                field,
                value: v.to_string(),
            }),
            other => Err(RecordParseError::UnexpectedType {
                field,
                value: format!("{other:?}"),
            }),
        }
    }
}

pub struct TfBoolParser;

impl Parser for TfBoolParser {
    type Output = bool;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            FieldValue::Character(Some(v)) => match v.trim() {
                "T" => Ok(true),
                "F" | "" => Ok(false),
                other => Err(RecordParseError::InvalidField {
                    field,
                    value: other.to_string(),
                }),
            },
            FieldValue::Character(None) => Ok(false),
            other => Err(RecordParseError::UnexpectedType {
                field,
                value: format!("{other:?}"),
            }),
        }
    }
}

use shapefile::ShapeType;
use shapefile::dbase::FieldValue;
use std::marker::PhantomData;
use std::str::FromStr;

pub struct FromStrParser<T>(PhantomData<T>);

impl<T> Parser for FromStrParser<T>
where
    T: FromStr,
{
    type Output = T;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        let s = StringParser::parse(field, value)?;
        T::from_str(&s).map_err(|_| RecordParseError::InvalidField { field, value: s })
    }
}

pub struct OptionParser<P>(std::marker::PhantomData<P>);

impl<P: Parser> Parser for OptionParser<P> {
    type Output = Option<P::Output>;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        match value {
            // Treat explicit NULL as None
            shapefile::dbase::FieldValue::Character(None)
            | shapefile::dbase::FieldValue::Numeric(None) => Ok(None),

            // Otherwise delegate
            _ => Ok(Some(P::parse(field, value)?)),
        }
    }
}

pub struct DefaultParser<P, const DEFAULT: bool>(PhantomData<P>);

impl<P: Parser<Output = bool>, const DEFAULT: bool> Parser for DefaultParser<P, DEFAULT> {
    type Output = bool;

    fn parse(field: &'static str, value: &FieldValue) -> Result<Self::Output, RecordParseError> {
        OptionParser::<P>::parse(field, value).map(|opt| opt.unwrap_or(DEFAULT))
    }
}

pub trait ShapefileElement: Sized {
    fn id(&self) -> i64;

    fn from_shapefile_item(
        item: (shapefile::Shape, &shapefile::dbase::Record),
    ) -> Result<Self, RecordParseError>;
}
