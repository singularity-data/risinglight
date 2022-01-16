use super::*;
use crate::binder::BoundCreateTable;
use crate::optimizer::plan_nodes::LogicalCreateTable;

impl LogicalPlaner {
    pub fn plan_create_table(&self, stmt: BoundCreateTable) -> Result<PlanRef, LogicalPlanError> {
        Ok(Rc::new(LogicalCreateTable::new(
            stmt.database_id,
            stmt.schema_id,
            stmt.table_name,
            stmt.columns,
        )))
    }
}
