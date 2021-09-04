use super::*;
use crate::types::{DataTypeEnum, DataValue};
use postgres_parser as pg;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, PartialEq)]
pub struct Expression {
    pub(crate) alias: Option<String>,
    pub(crate) data: ExprData,
}

#[derive(Debug, PartialEq)]
pub enum ExprData {
    Constant(DataValue),
    ColumnRef {
        /// Table name. If it's not set at the transforming time, we need to search
        /// for the corresponding table name within the binder context.
        table_name: Option<String>,
        /// Column name.
        column_name: String,
        // TODO: binder variables
    },
    /// A (*) in the SELECT clause.
    Star,
}

impl Expression {
    pub const fn constant(value: DataValue) -> Self {
        Expression {
            alias: None,
            data: ExprData::Constant(value),
        }
    }

    pub const fn star() -> Self {
        Expression {
            alias: None,
            data: ExprData::Star,
        }
    }

    pub const fn column_ref(column_name: String, table_name: Option<String>) -> Self {
        Expression {
            alias: None,
            data: ExprData::ColumnRef {
                table_name,
                column_name,
            },
        }
    }
}

impl TryFrom<&pg::Node> for Expression {
    type Error = ParseError;

    fn try_from(node: &pg::Node) -> Result<Self, Self::Error> {
        match node {
            pg::Node::ColumnRef(node) => node.try_into(),
            pg::Node::A_Const(node) => node.try_into(),
            _ => todo!("expression type"),
        }
    }
}

impl TryFrom<&pg::nodes::ColumnRef> for Expression {
    type Error = ParseError;

    fn try_from(node: &pg::nodes::ColumnRef) -> Result<Self, Self::Error> {
        match node.fields.as_ref().unwrap().as_slice() {
            [pg::Node::A_Star(_)] => Ok(Self::star()),
            [pg::Node::Value(v)] => {
                let column_name = v.string.as_ref().map(|s| s.to_lowercase()).unwrap();
                Ok(Self::column_ref(column_name, None))
            }
            [pg::Node::Value(v1), pg::Node::Value(v2)] => {
                let table_name = v1.string.as_ref().map(|s| s.to_lowercase());
                let column_name = v2.string.as_ref().map(|s| s.to_lowercase()).unwrap();
                Ok(Self::column_ref(column_name, table_name))
            }
            _ => todo!("unsupported column type"),
        }
    }
}

impl TryFrom<&pg::nodes::A_Const> for Expression {
    type Error = ParseError;

    fn try_from(node: &pg::nodes::A_Const) -> Result<Self, Self::Error> {
        Ok(Expression::constant(DataValue::try_from(&node.val)?))
    }
}

impl TryFrom<&pg::nodes::Value> for DataValue {
    type Error = ParseError;

    fn try_from(value: &pg::nodes::Value) -> Result<Self, Self::Error> {
        if value.null.is_some() {
            return Ok(DataValue::Null);
        }
        if let Some(v) = value.int {
            return Ok(DataValue::Int32(v));
        }
        if let Some(v) = &value.float {
            return Ok(DataValue::Float64(v.parse().unwrap()));
        }
        if let Some(v) = &value.string {
            return Ok(DataValue::String(v.clone()));
        }
        if let Some(v) = &value.bit_string {
            return Ok(DataValue::String(v.clone()));
        }
        Err(ParseError::InvalidInput("value"))
    }
}
