use std::fmt::Debug;
use super::TableSchema;

/// Table that stores a relation.
pub trait Table: Debug {

    /// Returns the number of rows in the table.
    fn row_num(&self) -> usize;

    /// Returns the schema of the table.
    fn schema(&self) -> &dyn TableSchema;

}