use std::fmt;

use super::*;

/// The physical plan of `EXPLAIN`.
#[derive(Debug, Clone)]
pub struct PhysicalExplain {
    pub plan: PlanRef,
}

impl_plan_tree_node!(PhysicalExplain, [plan]);
impl PlanNode for PhysicalExplain {}
impl fmt::Display for PhysicalExplain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "PhysicalExplain:")
    }
}
