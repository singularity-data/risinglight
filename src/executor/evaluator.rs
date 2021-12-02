use crate::{
    array::*,
    binder::BoundExpr,
    parser::{BinaryOperator, UnaryOperator},
    types::{ConvertError, DataTypeKind, DataValue},
};
use std::borrow::Borrow;

impl BoundExpr {
    /// Evaluate the given expression as a constant value.
    ///
    /// This method is used in the evaluation of `insert values` and optimizer
    pub fn eval(&self) -> DataValue {
        use DataValue::*;
        match &self {
            BoundExpr::Constant(v) => v.clone(),
            BoundExpr::UnaryOp(v) => match (&v.op, v.expr.eval()) {
                (UnaryOperator::Minus, Int32(i)) => Int32(-i),
                (UnaryOperator::Minus, Float64(f)) => Float64(-f),
                _ => todo!("evaluate expression: {:?}", self),
            },
            BoundExpr::BinaryOp(v) => match (&v.op, v.left_expr.eval(), v.right_expr.eval()) {
                (BinaryOperator::Plus, Int32(l), Int32(r)) => Int32(l + r),
                (BinaryOperator::Plus, Float64(l), Float64(r)) => Float64(l + r),
                (BinaryOperator::Minus, Int32(l), Int32(r)) => Int32(l - r),
                (BinaryOperator::Minus, Float64(l), Float64(r)) => Float64(l - r),
                (BinaryOperator::Multiply, Int32(l), Int32(r)) => Int32(l * r),
                (BinaryOperator::Multiply, Float64(l), Float64(r)) => Float64(l * r),
                (BinaryOperator::Divide, Int32(l), Int32(r)) => Int32(l / r),
                (BinaryOperator::Divide, Float64(l), Float64(r)) => Float64(l / r),
                _ => todo!("evaluate expression: {:?}", self),
            },
            _ => todo!("evaluate expression: {:?}", self),
        }
    }

    /// Evaluate the given expression as an array.
    pub fn eval_array(&self, chunk: &DataChunk) -> Result<ArrayImpl, ConvertError> {
        match &self {
            BoundExpr::InputRef(input_ref) => Ok(chunk.array_at(input_ref.index).clone()),
            BoundExpr::BinaryOp(binary_op) => {
                let left = binary_op.left_expr.eval_array(chunk)?;
                let right = binary_op.right_expr.eval_array(chunk)?;
                Ok(left.binary_op(&binary_op.op, &right))
            }
            BoundExpr::UnaryOp(op) => {
                let array = op.expr.eval_array(chunk)?;
                Ok(array.unary_op(&op.op))
            }
            BoundExpr::Constant(v) => {
                let mut builder = ArrayBuilderImpl::with_capacity(
                    chunk.cardinality(),
                    &self.return_type().unwrap(),
                );
                // TODO: optimize this
                for _ in 0..chunk.cardinality() {
                    builder.push(v);
                }
                Ok(builder.finish())
            }
            BoundExpr::TypeCast(cast) => {
                let array = cast.expr.eval_array(chunk)?;
                if self.return_type() == cast.expr.return_type() {
                    return Ok(array);
                }
                array.try_cast(cast.ty.clone())
            }
            BoundExpr::IsNull(expr) => {
                let array = expr.expr.eval_array(chunk)?;
                Ok(ArrayImpl::Bool(
                    (0..array.len())
                        .map(|i| array.get(i) == DataValue::Null)
                        .collect(),
                ))
            }
            _ => panic!("{:?} should not be evaluated in `eval_array`", self),
        }
    }
}

impl ArrayImpl {
    /// Perform unary operation.
    pub fn unary_op(&self, op: &UnaryOperator) -> ArrayImpl {
        type A = ArrayImpl;
        match op {
            UnaryOperator::Plus => match self {
                A::Int32(_) => self.clone(),
                A::Float64(_) => self.clone(),
                _ => panic!("+ can only be applied to Int or Float array"),
            },
            UnaryOperator::Minus => match self {
                A::Int32(a) => A::Int32(unary_op(a, |v| -v)),
                A::Float64(a) => A::Float64(unary_op(a, |v| -v)),
                _ => panic!("- can only be applied to Int or Float array"),
            },
            UnaryOperator::Not => match self {
                A::Bool(a) => A::Bool(unary_op(a, |b| !b)),
                _ => panic!("Not can only be applied to BOOL array"),
            },
            _ => todo!("evaluate operator: {:?}", op),
        }
    }

    /// Perform binary operation.
    pub fn binary_op(&self, op: &BinaryOperator, right: &ArrayImpl) -> ArrayImpl {
        type A = ArrayImpl;
        macro_rules! arith {
            ($op:tt) => {
                match (self, right) {
                    #[cfg(feature = "simd")]
                    (A::Int32(a), A::Int32(b)) => A::Int32(simd_op::<_, _, _, 32>(a, b, |a, b| a $op b)),
                    #[cfg(feature = "simd")]
                    (A::Float64(a), A::Float64(b)) => A::Float64(simd_op::<_, _, _, 32>(a, b, |a, b| a $op b)),

                    #[cfg(not(feature = "simd"))]
                    (A::Int32(a), A::Int32(b)) => A::Int32(binary_op(a, b, |a, b| a $op b)),
                    #[cfg(not(feature = "simd"))]
                    (A::Float64(a), A::Float64(b)) => A::Float64(binary_op(a, b, |a, b| a $op b)),
                    _ => todo!("Support more types for {}", stringify!($op)),
                }
            }
        }
        macro_rules! cmp {
            ($op:tt) => {
                match (self, right) {
                    (A::Bool(a), A::Bool(b)) => A::Bool(binary_op(a, b, |a, b| a $op b)),
                    (A::Int32(a), A::Int32(b)) => A::Bool(binary_op(a, b, |a, b| a $op b)),
                    #[allow(clippy::float_cmp)]
                    (A::Float64(a), A::Float64(b)) => A::Bool(binary_op(a, b, |a, b| a $op b)),
                    (A::Utf8(a), A::Utf8(b)) => A::Bool(binary_op(a, b, |a, b| a $op b)),
                    _ => todo!("Support more types for {}", stringify!($op)),
                }
            }
        }
        match op {
            BinaryOperator::Plus => arith!(+),
            BinaryOperator::Minus => arith!(-),
            BinaryOperator::Multiply => arith!(*),
            BinaryOperator::Divide => arith!(/),
            BinaryOperator::Modulo => arith!(%),
            BinaryOperator::Eq => cmp!(==),
            BinaryOperator::NotEq => cmp!(!=),
            BinaryOperator::Gt => cmp!(>),
            BinaryOperator::Lt => cmp!(<),
            BinaryOperator::GtEq => cmp!(>=),
            BinaryOperator::LtEq => cmp!(<=),
            BinaryOperator::And => match (self, right) {
                (A::Bool(a), A::Bool(b)) => {
                    A::Bool(binary_op_with_null(a, b, |a, b| match (a, b) {
                        (Some(a), Some(b)) => Some(*a && *b),
                        (Some(false), _) | (_, Some(false)) => Some(false),
                        _ => None,
                    }))
                }
                _ => panic!("And can only be applied to BOOL arrays"),
            },
            BinaryOperator::Or => match (self, right) {
                (A::Bool(a), A::Bool(b)) => {
                    A::Bool(binary_op_with_null(a, b, |a, b| match (a, b) {
                        (Some(a), Some(b)) => Some(*a || *b),
                        (Some(true), _) | (_, Some(true)) => Some(true),
                        _ => None,
                    }))
                }
                _ => panic!("Or can only be applied to BOOL arrays"),
            },
            _ => todo!("evaluate operator: {:?}", op),
        }
    }

    /// Cast the array to another type.
    pub fn try_cast(&self, data_type: DataTypeKind) -> Result<Self, ConvertError> {
        type Type = DataTypeKind;
        Ok(match self {
            Self::Bool(a) => match data_type {
                Type::Boolean => Self::Bool(a.clone()),
                Type::Int(_) => Self::Int32(unary_op(a, |&b| b as i32)),
                Type::Float(_) | Type::Double => Self::Float64(unary_op(a, |&b| b as u8 as f64)),
                Type::String | Type::Char(_) | Type::Varchar(_) => {
                    Self::Utf8(unary_op(a, |&b| if b { "true" } else { "false" }))
                }
                _ => todo!("cast array"),
            },
            Self::Int32(a) => match data_type {
                Type::Boolean => Self::Bool(unary_op(a, |&i| i != 0)),
                Type::Int(_) => Self::Int32(a.clone()),
                Type::Float(_) | Type::Double => Self::Float64(unary_op(a, |&i| i as f64)),
                Type::String | Type::Char(_) | Type::Varchar(_) => {
                    Self::Utf8(unary_op(a, |&i| i.to_string()))
                }
                _ => todo!("cast array"),
            },
            Self::Int64(a) => match data_type {
                Type::Boolean => Self::Bool(unary_op(a, |&i| i != 0)),
                Type::Int(_) => Self::Int64(a.clone()),
                Type::Float(_) | Type::Double => Self::Float64(unary_op(a, |&i| i as f64)),
                Type::String | Type::Char(_) | Type::Varchar(_) => {
                    Self::Utf8(unary_op(a, |&i| i.to_string()))
                }
                _ => todo!("cast array"),
            },
            Self::Float64(a) => match data_type {
                Type::Boolean => Self::Bool(unary_op(a, |&f| f != 0.0)),
                Type::Int(_) => Self::Int32(unary_op(a, |&f| f as i32)),
                Type::Float(_) | Type::Double => Self::Float64(a.clone()),
                Type::String | Type::Char(_) | Type::Varchar(_) => {
                    Self::Utf8(unary_op(a, |&f| f.to_string()))
                }
                _ => todo!("cast array"),
            },
            Self::Utf8(a) => match data_type {
                Type::Boolean => Self::Bool(try_unary_op(a, |s| {
                    s.parse::<bool>()
                        .map_err(|e| ConvertError::ParseBool(s.to_string(), e))
                })?),
                Type::Int(_) => Self::Int32(try_unary_op(a, |s| {
                    s.parse::<i32>()
                        .map_err(|e| ConvertError::ParseInt(s.to_string(), e))
                })?),
                Type::Float(_) | Type::Double => Self::Float64(try_unary_op(a, |s| {
                    s.parse::<f64>()
                        .map_err(|e| ConvertError::ParseFloat(s.to_string(), e))
                })?),
                Type::String | Type::Char(_) | Type::Varchar(_) => Self::Utf8(a.clone()),
                _ => todo!("cast array"),
            },
        })
    }
}

#[cfg(feature = "simd")]
use crate::types::NativeType;
#[cfg(feature = "simd")]
use std::simd::{LaneCount, Simd, SimdElement, SupportedLaneCount};

#[cfg(feature = "simd")]
pub fn simd_op<T, O, F, const N: usize>(
    a: &PrimitiveArray<T>,
    b: &PrimitiveArray<T>,
    f: F,
) -> PrimitiveArray<O>
where
    T: NativeType + SimdElement,
    O: NativeType + SimdElement,
    F: Fn(Simd<T, N>, Simd<T, N>) -> Simd<O, N>,
    LaneCount<N>: SupportedLaneCount,
{
    assert_eq!(a.len(), b.len());
    a.batch_iter::<N>()
        .zip(b.batch_iter::<N>())
        .map(|(a, b)| BatchItem {
            valid: a.valid & b.valid,
            data: f(a.data, b.data),
            len: a.len,
        })
        .collect()
}

pub fn binary_op<A, B, O, F, V>(a: &A, b: &B, f: F) -> O
where
    A: Array,
    B: Array,
    O: Array,
    V: Borrow<O::Item>,
    F: Fn(&A::Item, &B::Item) -> V,
{
    assert_eq!(a.len(), b.len());
    let mut builder = O::Builder::with_capacity(a.len());
    for (a, b) in a.iter().zip(b.iter()) {
        if let (Some(a), Some(b)) = (a, b) {
            builder.push(Some(f(a, b).borrow()));
        } else {
            builder.push(None);
        }
    }
    builder.finish()
}

fn binary_op_with_null<A, B, O, F, V>(a: &A, b: &B, f: F) -> O
where
    A: Array,
    B: Array,
    O: Array,
    V: Borrow<O::Item>,
    F: Fn(Option<&A::Item>, Option<&B::Item>) -> Option<V>,
{
    assert_eq!(a.len(), b.len());
    let mut builder = O::Builder::with_capacity(a.len());
    for (a, b) in a.iter().zip(b.iter()) {
        if let Some(c) = f(a, b) {
            builder.push(Some(c.borrow()));
        } else {
            builder.push(None);
        }
    }
    builder.finish()
}

fn unary_op<A, O, F, V>(a: &A, f: F) -> O
where
    A: Array,
    O: Array,
    V: Borrow<O::Item>,
    F: Fn(&A::Item) -> V,
{
    let mut builder = O::Builder::with_capacity(a.len());
    for e in a.iter() {
        if let Some(e) = e {
            builder.push(Some(f(e).borrow()));
        } else {
            builder.push(None);
        }
    }
    builder.finish()
}

fn try_unary_op<A, O, F, V, E>(a: &A, f: F) -> Result<O, E>
where
    A: Array,
    O: Array,
    V: Borrow<O::Item>,
    F: Fn(&A::Item) -> Result<V, E>,
{
    let mut builder = O::Builder::with_capacity(a.len());
    for e in a.iter() {
        if let Some(e) = e {
            builder.push(Some(f(e)?.borrow()));
        } else {
            builder.push(None);
        }
    }
    Ok(builder.finish())
}
