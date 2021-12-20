use std::fmt;

use super::*;
use crate::binder::Object;

/// The physical plan of `DROP`.
#[derive(Debug, Clone)]
pub struct PhysicalDrop {
    pub object: Object,
}

impl_plan_tree_node!(PhysicalDrop);
impl PlanNode for PhysicalDrop {}

impl fmt::Display for PhysicalDrop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:?}", self)
    }
}
