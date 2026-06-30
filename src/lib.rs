#![forbid(unsafe_code)]

mod utils;

mod connection;
mod error;
mod transaction;

pub mod data;
pub mod object;
pub mod storage;

pub use connection::Connection;
pub use data::ObjectId;
pub use error::{Result, Error};
pub use object::Object;
pub use transaction::{ObjectState, Transaction, Tx};

pub use orm_macro::Object;
