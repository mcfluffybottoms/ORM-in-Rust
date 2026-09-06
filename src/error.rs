use crate::{ObjectId, data::DataType};

use thiserror::Error;

//------------------------------------------------------------------------------//

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database is locked")]
    DatabaseLocked,
    #[error("Storage error: {0}.")]
    Storage(#[source] Box<dyn std::error::Error>),
    #[error(transparent)]
    NotFound(Box<NotFoundError>),
    #[error(transparent)]
    UnexpectedType(Box<UnexpectedTypeError>),
    #[error(transparent)]
    ColumnNotFound(Box<ColumnNotFoundError>),
}

pub type Result<T> = std::result::Result<T, Error>;

fn is_database_locked(err: &postgres::Error) -> bool {
    err.as_db_error()
        .map(|db_error| db_error.code() == &postgres::error::SqlState::LOCK_NOT_AVAILABLE)
        .unwrap_or(false)
}

impl From<postgres::Error> for Error {
    fn from(err: postgres::Error) -> Self {
        if is_database_locked(&err) {
            Error::DatabaseLocked
        } else {
            Error::Storage(Box::new(err))
        }
    }
}

#[derive(Error, Debug)]
#[error("Type mismatch for {type_name}::{attr_name}: expected {expected_type:?}, got {got_type} (table: {table_name}, column: {column_name})")]
pub struct UnexpectedTypeError {
    pub table_name: &'static str,
    pub column_name: &'static str,
    pub attr_name: &'static str,
    pub type_name: &'static str,
    pub expected_type: DataType,
    pub got_type: String,
}

#[derive(Error, Debug)]
#[error("Missing column `{column_name}` in table `{table_name}` for `{type_name}::{attr_name}`")]
pub struct ColumnNotFoundError {
    pub table_name: &'static str,
    pub column_name: &'static str,
    pub attr_name: &'static str,
    pub type_name: &'static str,
}

#[derive(Error, Debug)]
#[error("Object `{type_name}` with id {object_id:?} not found")]
pub struct NotFoundError {
    pub object_id: ObjectId,
    pub type_name: &'static str,
}
