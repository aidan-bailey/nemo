//! This module implements [Trie]
//! as well as its iterator [TrieScanGeneric].

use std::cell::UnsafeCell;

use crate::{
    columnar::{
        columnscan::ColumnScanRainbow,
        intervalcolumn::{
            interval_lookup::lookup_column_single::IntervalLookupColumnSingle, IntervalColumnT,
            IntervalColumnTBuilderMatrix, IntervalColumnTBuilderTriescan,
        },
    },
    datasources::tuple_writer::TupleWriter,
    datatypes::{StorageTypeName, StorageValueT},
};

use super::{
    buffer::{sorted_tuple_buffer::SortedTupleBuffer, tuple_buffer::TupleBuffer},
    triescan::{PartialTrieScan, TrieScan},
};

/// Defines the lookup method used in [IntervalColumnT]
type IntervalLookupMethod = IntervalLookupColumnSingle;

/// A tree like data structure for storing tabular data
///
/// A path in the tree from root to leaf corresponds
/// to a row in the represented table.
#[derive(Debug, Clone)]
pub struct Trie {
    /// Each [IntervalColumnT] represents one column in the table.
    /// We refer to each such column as a layer.
    columns: Vec<IntervalColumnT<IntervalLookupMethod>>,
}

impl Trie {
    /// Return the arity, that is the number of columns, in this trie.
    pub fn arity(&self) -> usize {
        self.columns.len()
    }

    /// Return a [PartialTrieScan] over this trie.
    pub fn iter(&self) -> TrieScanGeneric<'_> {
        let column_scans = self
            .columns
            .iter()
            .map(|column| UnsafeCell::new(column.iter()))
            .collect::<Vec<_>>();

        TrieScanGeneric::new(self, column_scans)
    }
}

impl Trie {
    pub(crate) fn from_tuple_buffer(buffer: SortedTupleBuffer) -> Self {
        let mut intervalcolumn_builders = (0..buffer.column_number())
            .map(|_| IntervalColumnTBuilderMatrix::<IntervalLookupMethod>::default())
            .collect::<Vec<_>>();

        let mut last_tuple_intervals = Vec::new();

        for (column_index, current_builder) in intervalcolumn_builders.iter_mut().enumerate() {
            let mut current_tuple_intervals = Vec::<usize>::new();

            let mut predecessor_index = 0;
            for (tuple_index, value) in buffer.get_column(column_index).enumerate() {
                if last_tuple_intervals.get(predecessor_index) == Some(&tuple_index) {
                    current_builder.finish_interval();
                    predecessor_index += 1;
                }

                let new_value = current_builder.add_value(value);

                if new_value && tuple_index > 0 {
                    current_tuple_intervals.push(tuple_index);
                }
            }

            current_builder.finish_interval();

            last_tuple_intervals = current_tuple_intervals;
        }

        Self {
            columns: intervalcolumn_builders
                .into_iter()
                .map(|builder| builder.finalize())
                .collect(),
        }
    }

    /// Create a new [Trie] from a [TupleWriter].
    pub fn from_tuple_writer(writer: TupleWriter) -> Self {
        Self::from_tuple_buffer(writer.finalize())
    }

    /// Create a new [Trie] based on an a [TrieScan].
    /// In the last `cut_layers` layers, this function will only check for the existence of a value
    /// and will not materialize it fully.
    /// To keep all the values, set `cut_layers` to 0.
    pub fn from_trie_scan<Scan: TrieScan>(mut trie_scan: Scan, cut_layers: usize) -> Self {
        let num_columns = trie_scan.num_columns() - cut_layers;

        let mut intervalcolumn_builders = (0..num_columns)
            .map(|_| IntervalColumnTBuilderTriescan::<IntervalLookupMethod>::default())
            .collect::<Vec<_>>();

        while let Some(changed_layer) = trie_scan.advance_on_layer(num_columns - 1) {
            for (layer, current_builder) in intervalcolumn_builders
                .iter_mut()
                .enumerate()
                .skip(changed_layer)
            {
                let current_value = trie_scan.current_value(layer);

                current_builder.add_value(current_value);
                if layer != changed_layer {
                    current_builder.finish_interval();
                }
            }
        }

        Self {
            columns: intervalcolumn_builders
                .into_iter()
                .map(|builder| builder.finalize())
                .collect(),
        }
    }

    /// Create a new [Trie] from a simple row based representation of the table.
    ///
    /// This function assumes that every row has the same number of entries.
    pub(crate) fn from_rows(rows: Vec<Vec<StorageValueT>>) -> Self {
        let column_number = if let Some(first_row) = rows.first() {
            first_row.len()
        } else {
            return Self {
                columns: Vec::new(),
            };
        };

        let mut tuple_buffer = TupleBuffer::new(column_number);

        for row in rows {
            debug_assert!(row.len() == column_number);

            for value in row {
                tuple_buffer.add_tuple_value(value);
            }
        }

        Self::from_tuple_buffer(tuple_buffer.finalize())
    }
}

/// Implementation of [PartialTrieScan] for a [Trie]
#[derive(Debug)]
pub struct TrieScanGeneric<'a> {
    /// Underlying [Trie] over which we are iterating
    trie: &'a Trie,

    /// Path of [StorageTypeName] indicating the the types of the current (partial) row
    path_types: Vec<StorageTypeName>,

    /// [ColumnScan] for each layer in the [PartialTrieScan]
    column_scans: Vec<UnsafeCell<ColumnScanRainbow<'a>>>,
}

impl<'a> TrieScanGeneric<'a> {
    /// Construct a new [TrieScanGeneric].
    pub fn new(trie: &'a Trie, column_scans: Vec<UnsafeCell<ColumnScanRainbow<'a>>>) -> Self {
        Self {
            trie,
            path_types: Vec::with_capacity(column_scans.len()),
            column_scans,
        }
    }
}

impl<'a> PartialTrieScan<'a> for TrieScanGeneric<'a> {
    fn up(&mut self) {
        debug_assert!(
            !self.path_types.is_empty(),
            "Attempted to go up in the starting position"
        );

        self.path_types.pop();
    }

    fn down(&mut self, next_type: StorageTypeName) {
        match self.path_types.last() {
            None => {
                self.column_scans[0].get_mut().reset(next_type);
            }
            Some(&current_type) => {
                let next_layer = self.path_types.len();
                let current_layer = next_layer - 1;

                let local_index = self.column_scans[current_layer]
                    .get_mut()
                    .pos(current_type)
                    .expect(
                        "Calling down on a trie is only allowed when currently pointing at an element.",
                    );
                let global_index =
                    self.trie.columns[current_layer].global_index(current_type, local_index);

                let next_interval = self.trie.columns[next_layer]
                    .interval_bounds(next_type, global_index)
                    .unwrap_or(0..0);

                self.column_scans[next_layer]
                    .get_mut()
                    .narrow(next_type, next_interval);
            }
        }

        self.path_types.push(next_type);
    }

    fn arity(&self) -> usize {
        self.trie.arity()
    }

    fn scan<'b>(&'b self, layer: usize) -> &'b UnsafeCell<ColumnScanRainbow<'a>> {
        &self.column_scans[layer]
    }

    fn path_types(&self) -> &[StorageTypeName] {
        &self.path_types
    }
}

#[cfg(test)]
mod test {
    use crate::{
        datatypes::{Float, StorageTypeName, StorageValueT},
        tabular::triescan::PartialTrieScan,
    };

    use super::{Trie, TrieScanGeneric};

    fn current_layer_next(
        scan: &mut TrieScanGeneric,
        storage_type: StorageTypeName,
    ) -> Option<StorageValueT> {
        let current_layer_scan = unsafe { &mut *scan.current_scan()?.get() };
        current_layer_scan.next(storage_type)
    }

    #[test]
    fn generic_trie_scan() {
        let table = vec![
            vec![
                StorageValueT::Id32(0),
                StorageValueT::Int64(-2),
                StorageValueT::Float(Float::new(1.2).unwrap()),
            ],
            vec![
                StorageValueT::Id32(0),
                StorageValueT::Int64(-1),
                StorageValueT::Id32(20),
            ],
            vec![
                StorageValueT::Id32(0),
                StorageValueT::Int64(-1),
                StorageValueT::Id32(32),
            ],
            vec![
                StorageValueT::Int64(6),
                StorageValueT::Id32(100),
                StorageValueT::Id32(101),
            ],
            vec![
                StorageValueT::Int64(6),
                StorageValueT::Id32(100),
                StorageValueT::Id32(102),
            ],
        ];

        let trie = Trie::from_rows(table);
        let mut scan = trie.iter();

        scan.down(StorageTypeName::Id32);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Id32),
            Some(StorageValueT::Id32(0))
        );
        scan.down(StorageTypeName::Id32);
        assert_eq!(current_layer_next(&mut scan, StorageTypeName::Id32), None);
        scan.up();
        scan.down(StorageTypeName::Id64);
        assert_eq!(current_layer_next(&mut scan, StorageTypeName::Id32), None);
        scan.up();
        scan.down(StorageTypeName::Int64);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Int64(-2))
        );
        scan.down(StorageTypeName::Float);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Float),
            Some(StorageValueT::Float(Float::new(1.2).unwrap()))
        );
        scan.up();
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Int64(-1))
        );
        scan.down(StorageTypeName::Float);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Float),
            Some(StorageValueT::Id32(20))
        );
        scan.down(StorageTypeName::Float);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Float),
            Some(StorageValueT::Id32(32))
        );
        scan.up();
        scan.up();
        scan.up();
        scan.down(StorageTypeName::Int64);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Int64(6))
        );
        scan.down(StorageTypeName::Id32);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Id32(100))
        );
        scan.down(StorageTypeName::Id32);
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Id32(101))
        );
        assert_eq!(
            current_layer_next(&mut scan, StorageTypeName::Int64),
            Some(StorageValueT::Id32(101))
        );
    }
}
