// import modules
mod utils;
mod transaction;
mod error;

// import types
pub mod data;
pub mod storage;
pub mod object;

// aliases for types
pub use storage::Connection;
pub use transaction::{ObjectState, Transaction, Tx};
pub use error::{Result, Error};
pub use object::Object;
pub use data::ObjectId;

// macros
pub use orm_macro::Object;
