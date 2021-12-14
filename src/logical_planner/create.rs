use super::*;
use crate::binder::BoundCreateTable;
use crate::logical_optimizer::plan_nodes::logical_create_table::LogicalCreateTable;

impl LogicalPlaner {
    pub fn plan_create_table(&self, stmt: BoundCreateTable) -> Result<Plan, LogicalPlanError> {
        Ok(Plan::LogicalCreateTable(LogicalCreateTable {
            database_id: stmt.database_id,
            schema_id: stmt.schema_id,
            table_name: stmt.table_name,
            columns: stmt.columns,
        }))
    }
}
