use super::*;
use crate::catalog::TableRefId;
use crate::logical_planner::LogicalSeqScan;
use crate::types::ColumnId;

/// The physical plan of sequential scan operation.
#[derive(Debug, PartialEq, Clone)]
pub struct PhysicalSeqScan {
    pub table_ref_id: TableRefId,
    pub column_ids: Vec<ColumnId>,
}

impl PhysicalPlaner {
    pub fn plan_seq_scan(&self, plan: LogicalSeqScan) -> Result<PhysicalPlan, PhysicalPlanError> {
        Ok(PhysicalPlan::SeqScan(PhysicalSeqScan {
            table_ref_id: plan.table_ref_id,
            column_ids: plan.column_ids,
        }))
    }
}
