use crate::data::{DataType, Value};

use std::any::Any;

//------------------------------------------------------------------------------//

pub trait Object: Any {
    fn schema() -> &'static Schema;
    fn as_row(&self) -> Row<'static>;
    fn as_object(row: &RowSlice) -> Self;
}

pub trait Store: Any {
    fn as_row(&self) -> Row<'static>;
    fn table_name(&self) -> &'static str;
    fn schema(&self) -> &Schema;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Object> Store for T {
    fn as_row(&self) -> Row<'static> {
        Object::as_row(self)
    }

    fn table_name(&self) -> &'static str {
        &T::schema().name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> &Schema {
        &T::schema()
    }
}

//------------------------------------------------------------------------------//

pub struct Field {
    pub name: &'static str,
    pub column_name: &'static str,
    pub field_type: DataType,
}

pub type Row<'a> = Vec<Value<'a>>;
pub type RowSlice<'a> = [Value<'a>];

pub struct Schema {
    pub name: &'static str,
    pub fields: &'static [Field],
    pub type_name: &'static str,
}
