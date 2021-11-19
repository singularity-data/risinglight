use super::{StatisticsGlobalAgg, StatisticsPartialAgg};
use crate::array::ArrayImpl;
use crate::storage::secondary::index::ColumnIndex;
use crate::types::DataValue;

pub struct DistinctValueGlobalAgg;

impl DistinctValueGlobalAgg {
    pub fn create() -> Self {
        Self
    }
}

impl StatisticsGlobalAgg for DistinctValueGlobalAgg {
    fn apply_batch(&mut self, _index: &ColumnIndex) {}

    fn get_output(&self) -> DataValue {
        DataValue::Int64(0)
    }
}

pub struct DistinctValuePartialAgg;

impl DistinctValuePartialAgg {
    pub fn create() -> Self {
        Self
    }
}

impl StatisticsPartialAgg for DistinctValuePartialAgg {
    fn apply_batch(&mut self, _index: &ArrayImpl) {}

    fn get_output(&self) -> DataValue {
        DataValue::Int64(0)
    }
}
