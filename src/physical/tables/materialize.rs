use super::{TableSchema, Trie, TrieScan, TrieScanEnum, TrieSchema, TrieSchemaEntry};
use crate::physical::columns::{
    AdaptiveColumnBuilder, AdaptiveColumnBuilderT, ColumnBuilder, ColumnScan,
    GenericIntervalColumn, IntervalColumnEnum, IntervalColumnT,
};
use crate::physical::datatypes::DataTypeName;

/// Given a TrieScan iterator, materialize its content into a trie
pub fn materialize(trie_scan: &mut TrieScanEnum) -> Trie {
    // Compute target schema (which is the same as the input schema...)
    // TODO: There should be a better way to clone something like this...
    let input_schema = trie_scan.get_schema();
    let mut target_attributes = Vec::<TrieSchemaEntry>::with_capacity(input_schema.arity());
    for var in 0..input_schema.arity() {
        target_attributes.push(TrieSchemaEntry {
            label: input_schema.get_label(var),
            datatype: input_schema.get_type(var),
        });
    }
    let target_schema = TrieSchema::new(target_attributes);

    // Setup column builders
    let mut result_columns = Vec::<IntervalColumnT>::with_capacity(target_schema.arity());
    let mut data_column_builders = Vec::<AdaptiveColumnBuilderT>::new();
    let mut intervals_column_builders = Vec::<AdaptiveColumnBuilder<usize>>::new();

    for var in 0..target_schema.arity() {
        intervals_column_builders.push(AdaptiveColumnBuilder::new());
        match input_schema.get_type(var) {
            DataTypeName::U64 => {
                data_column_builders.push(AdaptiveColumnBuilderT::U64(AdaptiveColumnBuilder::new()))
            }
            DataTypeName::Float => data_column_builders
                .push(AdaptiveColumnBuilderT::Float(AdaptiveColumnBuilder::new())),
            DataTypeName::Double => data_column_builders
                .push(AdaptiveColumnBuilderT::Double(AdaptiveColumnBuilder::new())),
        }
    }

    // Iterate through the trie_scan in a dfs manner
    let mut current_row: Vec<bool> = vec![false; target_schema.arity()];
    let mut current_int_starts: Vec<usize> = vec![0usize; target_schema.arity()];
    let mut current_layer: usize = 0;
    trie_scan.down();
    loop {
        let is_last_layer = current_layer >= target_schema.arity() - 1;
        let current_value = unsafe { (*trie_scan.current_scan().unwrap().get()).current() };
        let next_value = unsafe { (*trie_scan.current_scan().unwrap().get()).next() };

        if !current_row.last().unwrap() && is_last_layer {
            current_row = vec![true; target_schema.arity()];
        }

        if let Some(val) = current_value {
            if current_row[current_layer] {
                data_column_builders[current_layer].add(val);

                if !is_last_layer {
                    current_row[current_layer] = false;
                }
            }
        }

        if next_value.is_none() {
            let current_data_len = data_column_builders[current_layer].count();
            let prev_data_len = &mut current_int_starts[current_layer];

            if current_data_len > *prev_data_len {
                intervals_column_builders[current_layer].add(*prev_data_len);
                *prev_data_len = current_data_len;
            }

            if is_last_layer {
                current_row[current_layer] = false;
            }

            if current_layer == 0 {
                break;
            }

            trie_scan.up();
            current_layer -= 1;
            continue;
        }

        if !is_last_layer {
            trie_scan.down();
            current_layer += 1;
        }
    }

    // Collect data from column builders
    for _ in 0..target_schema.arity() {
        let current_data_builder: AdaptiveColumnBuilder<u64> =
            if let AdaptiveColumnBuilderT::U64(cb) = data_column_builders.remove(0) {
                cb
            } else {
                panic!("Only covering u64 for now");
            };
        let current_interval_builder = intervals_column_builders.remove(0);

        let next_interval_column = IntervalColumnT::U64(IntervalColumnEnum::GenericIntervalColumn(
            GenericIntervalColumn::new(
                current_data_builder.finalize(),
                current_interval_builder.finalize(),
            ),
        ));

        result_columns.push(next_interval_column);
    }

    // Finally, return finished trie
    Trie::new(target_schema, result_columns)
}

#[cfg(test)]
mod test {
    use super::materialize;
    use crate::physical::columns::{Column, IntervalColumnT};
    use crate::physical::datatypes::DataTypeName;
    use crate::physical::tables::{
        IntervalTrieScan, Trie, TrieJoin, TrieScanEnum, TrieSchema, TrieSchemaEntry,
    };
    use crate::physical::util::test_util::make_gict;
    use test_log::test;

    #[test]
    fn complete() {
        let column_fst_data = [1, 2, 3];
        let column_fst_int = [0];
        let column_snd_data = [2, 3, 4, 1, 2];
        let column_snd_int = [0, 2, 3];
        let column_trd_data = [3, 4, 5, 7, 2, 1];
        let column_trd_int = [0, 2, 3, 4, 5];

        let column_fst = make_gict(&column_fst_data, &column_fst_int);
        let column_snd = make_gict(&column_snd_data, &column_snd_int);
        let column_trd = make_gict(&column_trd_data, &column_trd_int);

        let column_vec = vec![column_fst, column_snd, column_trd];

        let schema = TrieSchema::new(vec![
            TrieSchemaEntry {
                label: 0,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 1,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 2,
                datatype: DataTypeName::U64,
            },
        ]);

        let trie = Trie::new(schema, column_vec);
        let mut trie_iter = TrieScanEnum::IntervalTrieScan(IntervalTrieScan::new(&trie));

        let materialized_trie = materialize(&mut trie_iter);

        let mat_in_col_fst = if let IntervalColumnT::U64(col) = materialized_trie.get_column(0) {
            col
        } else {
            panic!("Should be U64");
        };
        let mat_in_col_snd = if let IntervalColumnT::U64(col) = materialized_trie.get_column(1) {
            col
        } else {
            panic!("Should be U64");
        };
        let mat_in_col_trd = if let IntervalColumnT::U64(col) = materialized_trie.get_column(2) {
            col
        } else {
            panic!("Should be U64");
        };

        assert_eq!(
            mat_in_col_fst
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            column_fst_data
        );
        assert_eq!(
            mat_in_col_fst
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            column_fst_int
        );
        assert_eq!(
            mat_in_col_snd
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            column_snd_data
        );
        assert_eq!(
            mat_in_col_snd
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            column_snd_int
        );
        assert_eq!(
            mat_in_col_trd
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            column_trd_data
        );
        assert_eq!(
            mat_in_col_trd
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            column_trd_int
        );
    }

    #[test]
    fn partial() {
        // Same setup as in test_trie_join
        let column_a_x = make_gict(&[1, 2, 3], &[0]);
        let column_a_y = make_gict(&[2, 3, 4, 5, 6, 7], &[0, 3, 4]);
        let column_b_y = make_gict(&[1, 2, 3, 6], &[0]);
        let column_b_z = make_gict(&[1, 8, 9, 10, 11, 12], &[0, 1, 3, 4]);

        let schema_a = TrieSchema::new(vec![
            TrieSchemaEntry {
                label: 0,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 1,
                datatype: DataTypeName::U64,
            },
        ]);
        let schema_b = TrieSchema::new(vec![
            TrieSchemaEntry {
                label: 1,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 2,
                datatype: DataTypeName::U64,
            },
        ]);

        let schema_target = TrieSchema::new(vec![
            TrieSchemaEntry {
                label: 0,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 1,
                datatype: DataTypeName::U64,
            },
            TrieSchemaEntry {
                label: 2,
                datatype: DataTypeName::U64,
            },
        ]);

        let trie_a = Trie::new(schema_a, vec![column_a_x, column_a_y]);
        let trie_b = Trie::new(schema_b, vec![column_b_y, column_b_z]);

        let mut join_iter = TrieScanEnum::TrieJoin(TrieJoin::new(
            vec![
                TrieScanEnum::IntervalTrieScan(IntervalTrieScan::new(&trie_a)),
                TrieScanEnum::IntervalTrieScan(IntervalTrieScan::new(&trie_b)),
            ],
            schema_target,
        ));

        let materialized_join = materialize(&mut join_iter);

        let mat_in_col_fst = if let IntervalColumnT::U64(col) = materialized_join.get_column(0) {
            col
        } else {
            panic!("Should be U64");
        };
        let mat_in_col_snd = if let IntervalColumnT::U64(col) = materialized_join.get_column(1) {
            col
        } else {
            panic!("Should be U64");
        };
        let mat_in_col_trd = if let IntervalColumnT::U64(col) = materialized_join.get_column(2) {
            col
        } else {
            panic!("Should be U64");
        };

        assert_eq!(
            mat_in_col_fst
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            vec![1, 3]
        );
        assert_eq!(
            mat_in_col_fst
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            vec![0]
        );
        assert_eq!(
            mat_in_col_snd
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            vec![2, 3, 6]
        );
        assert_eq!(
            mat_in_col_snd
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            vec![0, 2]
        );
        assert_eq!(
            mat_in_col_trd
                .get_data_column()
                .iter()
                .collect::<Vec<u64>>(),
            vec![8, 9, 10, 11, 12]
        );
        assert_eq!(
            mat_in_col_trd
                .get_int_column()
                .iter()
                .collect::<Vec<usize>>(),
            vec![0, 2, 3]
        );
    }
}