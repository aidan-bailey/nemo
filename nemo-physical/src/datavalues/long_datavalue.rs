use super::{DataValue,ValueDomain};

/// Physical representation of an integer as an i64.
#[derive(Debug, Clone, Copy)]
pub struct Long(i64);

impl DataValue for Long {
    fn datatype_iri(&self) -> String {
        match self.value_domain() {
            ValueDomain::Long => "http://www.w3.org/2001/XMLSchema#long".to_owned(),
            ValueDomain::Int => "http://www.w3.org/2001/XMLSchema#int".to_owned(),
            _ => panic!("Unexpected value domain for i64"),
        }
    }

    fn lexical_value(&self) -> String {
        self.0.to_string()
    }

    /// The function needs to find the tightest domain for the given value.
    fn value_domain(&self) -> ValueDomain {
        if self.0 <= std::i32::MAX.into() && self.0 >= std::i32::MIN.into() {
            ValueDomain::Int
        } else {
            ValueDomain::Long
        }
    }

    fn to_i64(&self) -> i64 {
        self.0
    }

    fn to_i32(&self) -> i32 {
        // TODO: Maybe give a more informative error message here.
        self.0.try_into().unwrap()
    }
}