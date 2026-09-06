use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Lit, parse_macro_input};

#[proc_macro_derive(Object, attributes(table_name, column_name))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = get_struct_name(&input);
    let table_name = get_table_name(&input);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named.named.iter().collect::<Vec<_>>(),
            Fields::Unit => vec![],
            _ => panic!("Object derive only supports named or unit structs."),
        },
        _ => panic!("Object derive only works on structs."),
    };

    // get field info
    let extracted: Vec<_> = fields
        .iter()
        .map(|field| {
            let ident: &syn::Ident = field.ident.as_ref().unwrap();
            let column_name = get_column_name(field);
            let type_str = map_field_type_to_data_type_str(&field.ty);
            eprintln!("{}", get_column_name(field));
            (ident, column_name, type_str)
        })
        .collect();

    // orm::Field definitions
    let field_defs = extracted
        .iter()
        .map(|(ident, column, ty)| {
            let ident_str = ident.to_string();
            let data_type = match *ty {
                "String" => quote! { crate::DataType::String },
                "Bytes" => quote! { crate::DataType::Bytes },
                "Int64" => quote! { crate::DataType::Int64 },
                "Float64" => quote! { crate::DataType::Float64 },
                "Bool" => quote! { crate::DataType::Bool },
                _ => panic!("Unsupported data type: {}", ty),
            };
            quote! {
                crate::Field {
                    name: #ident_str,
                    column_name: #column,
                    field_type: #data_type,
                }
            }
        })
        .collect::<Vec<_>>();

    // value conversion for as_row()
    let field_conversions = extracted.iter().map(|(ident, _, ty)| match *ty {
        "Int64" => quote! { orm::data::Value::Int64(self.#ident) },
        "Float64" => quote! { orm::data::Value::Float64(self.#ident) },
        "String" => quote! { orm::data::Value::String(self.#ident.clone().into()) },
        "Bytes" => quote! { orm::data::Value::Bytes(self.#ident.clone().into()) },
        "Bool" => quote! { orm::data::Value::Bool(self.#ident) },
        _ => panic!("Unsupported type"),
    });

    // value conversion for as_row()
    let construct_fields = extracted.iter().map(|(ident, _, ty)|{
        let getter = match *ty {
            "String" => quote! {
                match &_row[index] {
                    Value::String(s) => s.to_string(),
                    _ => panic!("Expected String at column {}", index),
                }
            },
            "Bytes" => quote! {
                match &_row[index] {
                    Value::Bytes(b) => b.to_vec(),
                    _ => panic!("Expected Bytes at column {}", index),
                }
            },
            "Int64" => quote! {
                match &_row[index] {
                    Value::Int64(v) => *v,
                    _ => panic!("Expected Int64 at column {}", index),
                }
            },
            "Float64" => quote! {
                match &_row[index] {
                    Value::Float64(v) => *v,
                    _ => panic!("Expected Float64 at column {}", index),
                }
            },
            "Bool" => quote! {
                match &_row[index] {
                    Value::Bool(v) => *v,
                    _ => panic!("Expected Bool at column {}", index),
                }
            },
            other => panic!("Unsupported type {}", other),
        };

        quote! {
            #ident: {
                let value = #getter;
                index += 1;
                value
            }
        }
    });

    quote! {
        impl orm::Object for #struct_name {
            fn schema() -> &'static orm::object::Schema {

                static SCHEMA: orm::object::Schema = orm::object::Schema {
                    name: #table_name,
                    fields: &[
                        #(#field_defs),*
                    ],
                    type_name: #struct_name_str
                };
                &SCHEMA
            }

            fn as_row(&self) -> Row<'static> {
                vec![
                    #(#field_conversions),*
                ]
            }

            fn as_object(_row: &RowSlice) -> Self {
                let mut index = 0;
                Self {
                    #(#construct_fields),*
                }
            }
        }
    }.into()
}

fn get_table_name(input: &DeriveInput) -> String {
    for attr in &input.attrs {
        if attr.path.is_ident("table_name") {
            if let Ok(lit) = attr.parse_args::<Lit>() {
                if let Lit::Str(lit_str) = lit {
                    return lit_str.value();
                }
            }
        }
    }
    input.ident.to_string()
}

fn get_column_name(field: &syn::Field) -> String {
    for attr in &field.attrs {
        if attr.path.is_ident("column_name") {
            if let Ok(lit) = attr.parse_args::<Lit>() {
                if let Lit::Str(lit_str) = lit {
                    return lit_str.value();
                }
            }
        }
    }
    field.ident.as_ref().unwrap().to_string()
}

fn get_struct_name(input: &DeriveInput) -> String {
    String::from(&input.ident.to_string())
}

fn map_field_type_to_data_type_str(ty: &syn::Type) -> &'static str {
    use syn::Type;

    let segment = match ty {
        Type::Path(type_path) => type_path.path.segments.last().unwrap(),
        _ => panic!("Unsupported type"),
    };

    let ident = &segment.ident;

    match ident.to_string().as_str() {
        "i64" | "i32" | "u64" | "u32" | "isize" | "usize" => "Int64",
        "f64" | "f32" => "Float64",
        "String" => "String",
        "bool" => "Bool",
        "Vec" => "Bytes",
        other => panic!("Unsupported type: {}", other),
    }
}
