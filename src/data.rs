use std::borrow::Cow;

//------------------------------------------------------------------------------//

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjectId(pub(crate) i64);

impl ObjectId {
    pub fn new(v: i64) -> Self {
        ObjectId(v)
    }
    pub fn get(&self) -> i64 {
        self.0
    }
    pub fn into_i64(&self) -> i64 {
        self.0
    }
}

impl Into<ObjectId> for i64 {
    fn into(self) -> ObjectId {
        ObjectId::new(self)
    }
}

//------------------------------------------------------------------------------//

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    String,
    Bytes,
    Int64,
    Float64,
    Bool,
    Null
}

//------------------------------------------------------------------------------//

pub enum Value<'a> {
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Int64(i64),
    Float64(f64),
    Bool(bool),
}

impl<'a> From<i64> for Value<'a> {
    fn from(val: i64) -> Self {
        Value::Int64(val)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(val: f64) -> Self {
        Value::Float64(val)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(val: bool) -> Self {
        Value::Bool(val)
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(val: String) -> Self {
        Value::String(val.into())
    }
}

impl<'a> From<Vec<u8>> for Value<'a> {
    fn from(val: Vec<u8>) -> Self {
        Value::Bytes(val.into())
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(val: &'a str) -> Self {
        Value::String(Cow::Borrowed(val))
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(val: &'a [u8]) -> Self {
        Value::Bytes(val.into())
    }
}

pub fn sql_type(data_type: &DataType) -> &'static str {
    match data_type {
        DataType::String => "TEXT",
        DataType::Bytes => "BLOB",
        DataType::Int64 => "INTEGER",
        DataType::Float64 => "REAL",
        DataType::Bool => "TINYINT",
        DataType::Null => "NULL",
    }
}