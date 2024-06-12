// Copyright 2024 RisingLight Project Authors. Licensed under Apache-2.0.

use crate::types::DataType;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FunctionCatalog {
    pub name: String,
    pub arg_types: Vec<DataType>,
    pub arg_names: Vec<String>,
    pub return_type: DataType,
    pub language: String,
    pub body: String,
    pub is_recursive: bool,
}

impl FunctionCatalog {
    pub fn new(
        name: String,
        arg_types: Vec<DataType>,
        arg_names: Vec<String>,
        return_type: DataType,
        language: String,
        body: String,
        is_recursive: bool,
    ) -> Self {
        Self {
            name,
            arg_types,
            arg_names,
            return_type,
            language,
            body,
            is_recursive,
        }
    }

    #[inline]
    pub fn body(&self) -> String {
        self.body.clone()
    }

    #[inline]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[inline]
    pub fn language(&self) -> String {
        self.language.clone()
    }
}
