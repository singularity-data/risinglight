// Copyright 2022 RisingLight Project Authors. Licensed under Apache-2.0.

use std::fmt;

use serde::Serialize;

use super::*;
use crate::binder::{BoundAggCall, BoundExpr};
use crate::optimizer::logical_plan_rewriter::ExprRewriter;

/// The logical plan of hash aggregate operation.
#[derive(Debug, Clone, Serialize)]
pub struct LogicalAggregate {
    agg_calls: Vec<BoundAggCall>,
    /// Group keys in hash aggregation (optional)
    group_keys: Vec<BoundExpr>,
    child: PlanRef,
}

impl LogicalAggregate {
    pub fn new(agg_calls: Vec<BoundAggCall>, group_keys: Vec<BoundExpr>, child: PlanRef) -> Self {
        LogicalAggregate {
            agg_calls,
            group_keys,
            child,
        }
    }

    /// Get a reference to the logical aggregate's agg calls.
    pub fn agg_calls(&self) -> &[BoundAggCall] {
        self.agg_calls.as_ref()
    }

    /// Get a reference to the logical aggregate's group keys.
    pub fn group_keys(&self) -> &[BoundExpr] {
        self.group_keys.as_ref()
    }

    pub fn clone_with_rewrite_expr(
        &self,
        new_child: PlanRef,
        rewriter: &impl ExprRewriter,
    ) -> Self {
        let mut new_agg_calls = self.agg_calls().to_vec();
        let mut new_keys = self.group_keys().to_vec();
        for agg in &mut new_agg_calls {
            for arg in &mut agg.args {
                rewriter.rewrite_expr(arg);
            }
        }
        for keys in &mut new_keys {
            rewriter.rewrite_expr(keys);
        }

        LogicalAggregate::new(new_agg_calls, new_keys, new_child)
    }
}

impl PlanTreeNodeUnary for LogicalAggregate {
    fn child(&self) -> PlanRef {
        self.child.clone()
    }
    #[must_use]
    fn clone_with_child(&self, child: PlanRef) -> Self {
        Self::new(self.agg_calls().to_vec(), self.group_keys().to_vec(), child)
    }
}
impl_plan_tree_node_for_unary!(LogicalAggregate);
impl PlanNode for LogicalAggregate {
    fn schema(&self) -> Vec<ColumnDesc> {
        let child_schema = self.child.schema();
        self.group_keys
            .iter()
            .map(|expr| match expr {
                BoundExpr::InputRef(input_ref) => child_schema[input_ref.index].clone(),
                _ => panic!("group key should be an input ref"),
            })
            .chain(self.agg_calls.iter().map(|agg_call| {
                use crate::binder::AggKind::*;
                let name = match agg_call.kind {
                    Avg => "avg",
                    RowCount | Count => "count",
                    Max => "max",
                    Min => "min",
                    Sum => "sum",
                }
                .to_string();
                agg_call.return_type.clone().to_column(name)
            }))
            .collect()
    }
}
impl fmt::Display for LogicalAggregate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "LogicalAggregate: {} agg calls", self.agg_calls.len(),)
    }
}
