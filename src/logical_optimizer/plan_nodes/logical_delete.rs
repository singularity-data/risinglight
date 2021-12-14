use std::fmt;

use super::{impl_plan_tree_node_for_unary, Plan, PlanRef, PlanTreeNode};
use crate::catalog::TableRefId;
use crate::logical_optimizer::plan_nodes::UnaryLogicalPlanNode;

/// The logical plan of `delete`.
#[derive(Debug, PartialEq, Clone)]
pub struct LogicalDelete {
    pub table_ref_id: TableRefId,
    pub child: PlanRef,
}

impl UnaryLogicalPlanNode for LogicalDelete {
    fn child(&self) -> PlanRef {
        self.child.clone()
    }

    fn clone_with_child(&self, child: PlanRef) -> PlanRef {
        Plan::LogicalDelete(LogicalDelete {
            table_ref_id: self.table_ref_id,
            child,
        })
        .into()
    }
}
impl_plan_tree_node_for_unary! {LogicalDelete}

impl fmt::Display for LogicalDelete {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "PhysicalDelete: table {}", self.table_ref_id.table_id)
    }
}
