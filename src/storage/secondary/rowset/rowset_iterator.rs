use std::sync::Arc;

use bitvec::bitvec;
use bitvec::prelude::{BitVec, Lsb0};
use smallvec::smallvec;

use super::super::{
    ColumnIteratorImpl, ColumnSeekPosition, RowHandlerSequencer, SecondaryIteratorImpl,
};
use super::DiskRowset;
use crate::array::{Array, ArrayImpl};
use crate::binder::BoundExpr;
use crate::storage::secondary::DeleteVector;
use crate::storage::{PackedVec, StorageChunk, StorageColumnRef};

/// When `expected_size` is not specified, we should limit the maximum size of the chunk.
const ROWSET_MAX_OUTPUT: usize = 65536;

/// Iterates on a `RowSet`
pub struct RowSetIterator {
    rowset: Arc<DiskRowset>,
    column_refs: Arc<[StorageColumnRef]>,
    dvs: Vec<Arc<DeleteVector>>,
    column_iterators: Vec<Option<ColumnIteratorImpl>>,
    filter_expr: Option<(BoundExpr, BitVec)>,
}

impl RowSetIterator {
    pub async fn new(
        rowset: Arc<DiskRowset>,
        column_refs: Arc<[StorageColumnRef]>,
        dvs: Vec<Arc<DeleteVector>>,
        seek_pos: ColumnSeekPosition,
        expr: Option<BoundExpr>,
    ) -> Self {
        let start_row_id = match seek_pos {
            ColumnSeekPosition::RowId(row_id) => row_id,
            _ => todo!(),
        };

        if column_refs.len() == 0 {
            panic!("no column to iterate")
        }

        let row_handler_count = column_refs
            .iter()
            .filter(|x| matches!(x, StorageColumnRef::RowHandler))
            .count();

        if row_handler_count > 1 {
            panic!("more than 1 row handler column")
        }

        if row_handler_count == column_refs.len() {
            panic!("no user column")
        }

        let mut column_iterators: Vec<Option<ColumnIteratorImpl>> = vec![];

        for column_ref in &*column_refs {
            // TODO: parallel seek
            match column_ref {
                StorageColumnRef::RowHandler => column_iterators.push(None),
                StorageColumnRef::Idx(idx) => column_iterators.push(Some(
                    ColumnIteratorImpl::new(
                        rowset.column(*idx as usize),
                        rowset.column_info(*idx as usize),
                        start_row_id,
                    )
                    .await,
                )),
            };
        }

        let filter_expr = if let Some(expr) = expr {
            let filter_column = expr.get_filter_column(column_refs.len());
            Some((expr, filter_column))
        } else {
            None
        };

        Self {
            rowset,
            column_iterators,
            dvs,
            column_refs,
            filter_expr,
        }
    }

    pub async fn next_batch_inner_with_filter(
        &mut self,
        expected_size: Option<usize>,
    ) -> (bool, Option<StorageChunk>) {
        let (expr, filter_column) = self.filter_expr.as_ref().unwrap();

        let fetch_size = if let Some(x) = expected_size {
            x
        } else {
            // When `expected_size` is not available, we try to dispatch
            // as little I/O as possible. We find the minimum fetch hints
            // from the column iterators.
            let mut min = None;
            for it in self.column_iterators.iter().flatten() {
                let hint = it.fetch_hint();
                if hint != 0 {
                    if min.is_none() {
                        min = Some(hint);
                    } else {
                        min = Some(min.unwrap().min(hint));
                    }
                }
            }
            min.unwrap_or(ROWSET_MAX_OUTPUT)
        };

        let mut arrays: PackedVec<Option<ArrayImpl>> = smallvec![];
        let mut common_chunk_range = None;

        // TODO: parallel fetch
        // TODO: align unmatched rows

        for id in 0..filter_column.len() {
            let flag = filter_column[id];
            if flag {
                if let Some((row_id, array)) = self.column_iterators[id]
                    .as_mut()
                    .unwrap()
                    .next_batch(Some(fetch_size), None)
                    .await
                {
                    if let Some(x) = common_chunk_range {
                        if x != (row_id, array.len()) {
                            panic!("unmatched rowid from column iterator");
                        }
                    }
                    common_chunk_range = Some((row_id, array.len()));
                    arrays.push(Some(array));
                } else {
                    arrays.push(None);
                }
            } else {
                arrays.push(None);
            }
        }

        if common_chunk_range.is_none() {
            return (true, None);
        }

        // Need to rewrite and optimize
        let bool_array = match expr.eval_array_in_storage(&arrays).unwrap() {
            ArrayImpl::Bool(a) => a,
            _ => panic!("filters can only accept bool array"),
        };
        let mut filter_bitmap = bitvec![];
        for i in bool_array.iter() {
            if let Some(i) = i {
                filter_bitmap.push(*i);
            } else {
                filter_bitmap.push(false);
            }
        }

        // Use filter_bitmap to filter columns
        for (id, column_ref) in self.column_refs.iter().enumerate() {
            match column_ref {
                StorageColumnRef::RowHandler => continue,
                StorageColumnRef::Idx(_) => {
                    if matches!(arrays[id], None) {
                        if let Some((row_id, array)) = self.column_iterators[id]
                            .as_mut()
                            .unwrap()
                            .next_batch(Some(fetch_size), Some(&filter_bitmap))
                            .await
                        {
                            if let Some(x) = common_chunk_range {
                                if x != (row_id, array.len()) {
                                    println!(
                                        "filter_column_row_id: {}   normal_column_row_id: {}",
                                        x.0, row_id
                                    );
                                    println!(
                                        "filter_column_len: {}   normal_column_len: {}",
                                        x.1,
                                        array.len()
                                    );
                                    panic!("unmatched rowid from column iterator");
                                }
                            }
                            common_chunk_range = Some((row_id, array.len()));
                            arrays[id] = Some(array);
                        }
                    }
                }
            }
        }

        let common_chunk_range = if let Some(common_chunk_range) = common_chunk_range {
            common_chunk_range
        } else {
            return (true, None);
        };

        // Fill RowHandlers
        for (id, column_ref) in self.column_refs.iter().enumerate() {
            if matches!(column_ref, StorageColumnRef::RowHandler) {
                arrays[id] = Some(
                    RowHandlerSequencer::sequence(
                        self.rowset.rowset_id(),
                        common_chunk_range.0,
                        common_chunk_range.1 as u32,
                    )
                    .into(),
                );
            }
        }

        // Generate visibility bitmap
        let visibility = if self.dvs.is_empty() {
            Some(filter_bitmap)
        } else {
            let mut vis = BitVec::new();
            vis.resize(common_chunk_range.1, true);
            for dv in &self.dvs {
                dv.apply_to(&mut vis, common_chunk_range.0);
            }
            vis &= filter_bitmap;
            Some(vis)
        };

        (
            false,
            StorageChunk::construct(
                visibility,
                arrays
                    .into_iter()
                    .map(Option::unwrap)
                    .map(Arc::new)
                    .collect(),
            ),
        )
    }

    /// Return (finished, data chunk of the current iteration)
    ///
    /// It is possible that after applying the deletion map, the current data chunk contains no
    /// element. In this case, the chunk will not be returned to the upper layer.
    ///
    /// TODO: check the deletion map before actually fetching data from column iterators.
    pub async fn next_batch_inner(
        &mut self,
        expected_size: Option<usize>,
    ) -> (bool, Option<StorageChunk>) {
        let fetch_size = if let Some(x) = expected_size {
            x
        } else {
            // When `expected_size` is not available, we try to dispatch
            // as little I/O as possible. We find the minimum fetch hints
            // from the column itertaors.
            let mut min = None;
            for it in self.column_iterators.iter().flatten() {
                let hint = it.fetch_hint();
                if hint != 0 {
                    if min.is_none() {
                        min = Some(hint);
                    } else {
                        min = Some(min.unwrap().min(hint));
                    }
                }
            }
            min.unwrap_or(ROWSET_MAX_OUTPUT)
        };

        let mut arrays: PackedVec<Option<ArrayImpl>> = smallvec![];
        let mut common_chunk_range = None;

        // TODO: parallel fetch
        // TODO: align unmatched rows

        // Fill column data
        for (id, column_ref) in self.column_refs.iter().enumerate() {
            match column_ref {
                StorageColumnRef::RowHandler => arrays.push(None),
                StorageColumnRef::Idx(_) => {
                    if let Some((row_id, array)) = self.column_iterators[id]
                        .as_mut()
                        .unwrap()
                        .next_batch(Some(fetch_size), None)
                        .await
                    {
                        if let Some(x) = common_chunk_range {
                            if x != (row_id, array.len()) {
                                panic!("unmatched rowid from column iterator");
                            }
                        }
                        common_chunk_range = Some((row_id, array.len()));
                        arrays.push(Some(array));
                    } else {
                        arrays.push(None);
                    }
                }
            }
        }

        let common_chunk_range = if let Some(common_chunk_range) = common_chunk_range {
            common_chunk_range
        } else {
            return (true, None);
        };

        // Fill RowHandlers
        for (id, column_ref) in self.column_refs.iter().enumerate() {
            if matches!(column_ref, StorageColumnRef::RowHandler) {
                arrays[id] = Some(
                    RowHandlerSequencer::sequence(
                        self.rowset.rowset_id(),
                        common_chunk_range.0,
                        common_chunk_range.1 as u32,
                    )
                    .into(),
                );
            }
        }

        // Generate visibility bitmap
        let visibility = if self.dvs.is_empty() {
            None
        } else {
            let mut vis = BitVec::new();
            vis.resize(common_chunk_range.1, true);
            for dv in &self.dvs {
                dv.apply_to(&mut vis, common_chunk_range.0);
            }
            Some(vis)
        };

        (
            false,
            StorageChunk::construct(
                visibility,
                arrays
                    .into_iter()
                    .map(Option::unwrap)
                    .map(Arc::new)
                    .collect(),
            ),
        )
    }

    pub async fn next_batch(&mut self, expected_size: Option<usize>) -> Option<StorageChunk> {
        loop {
            let (finished, batch) = if self.filter_expr.is_some() {
                self.next_batch_inner_with_filter(expected_size).await
            } else {
                self.next_batch_inner(expected_size).await
            };
            if finished {
                return None;
            } else if let Some(batch) = batch {
                return Some(batch);
            }
        }
    }
}

impl SecondaryIteratorImpl for RowSetIterator {}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    use crate::array::{Array, ArrayToVecExt};
    use crate::storage::secondary::rowset::tests::helper_build_rowset;
    use crate::storage::secondary::SecondaryRowHandler;

    #[tokio::test]
    async fn test_rowset_iterator() {
        let tempdir = tempfile::tempdir().unwrap();
        let rowset = Arc::new(helper_build_rowset(&tempdir, false, 1000).await);
        let mut it = rowset
            .iter(
                vec![
                    StorageColumnRef::RowHandler,
                    StorageColumnRef::Idx(2),
                    StorageColumnRef::Idx(0),
                ]
                .into(),
                vec![],
                ColumnSeekPosition::RowId(1000),
                None,
            )
            .await;
        let chunk = it.next_batch(Some(1000)).await.unwrap();
        if let ArrayImpl::Int32(array) = chunk.array_at(2).as_ref() {
            let left = array.to_vec();
            let right = [1, 2, 3]
                .iter()
                .cycle()
                .cloned()
                .take(1000)
                .map(Some)
                .collect_vec();
            assert_eq!(left.len(), right.len());
            assert_eq!(left, right);
        } else {
            unreachable!()
        }

        if let ArrayImpl::Int32(array) = chunk.array_at(1).as_ref() {
            let left = array.to_vec();
            let right = [2, 3, 3, 3, 3, 3, 3]
                .iter()
                .cycle()
                .cloned()
                .take(1000)
                .map(Some)
                .collect_vec();
            assert_eq!(left.len(), right.len());
            assert_eq!(left, right);
        } else {
            unreachable!()
        }

        if let ArrayImpl::Int64(array) = chunk.array_at(0).as_ref() {
            assert_eq!(array.get(0), Some(&SecondaryRowHandler(0, 1000).as_i64()))
        } else {
            unreachable!()
        }
    }
}
