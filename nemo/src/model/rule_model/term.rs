use std::fmt::{Debug, Display};

use nemo_physical::datavalues::AnyDataValue;

use crate::{
    error::Error,
    model::{types::primitive_logical_value::LOGICAL_NULL_PREFIX, VariableAssignment},
};

use super::{Aggregate, Identifier, Map, NumericLiteral, RdfLiteral, Tuple};

/// Variable that can be bound to a specific value
#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub enum Variable {
    /// A universally quantified variable.
    Universal(Identifier),
    /// An existentially quantified variable.
    Existential(Identifier),
}

impl Variable {
    const WILDCARD_PREFIX: &'static str = "__WILDCARD";

    /// Return the name of the variable.
    pub fn name(&self) -> String {
        match self {
            Self::Universal(identifier) | Self::Existential(identifier) => identifier.name(),
        }
    }

    /// Return whether this is a universal variable.
    pub fn is_universal(&self) -> bool {
        matches!(self, Variable::Universal(_))
    }

    /// Return whether this is an existential variable.
    pub fn is_existential(&self) -> bool {
        matches!(self, Variable::Existential(_))
    }

    /// Return whether this variable was generated by a wildcard pattern.
    pub fn is_wildcard(&self) -> bool {
        match self {
            Variable::Universal(ident) => ident.0.starts_with(Self::WILDCARD_PREFIX),
            Variable::Existential(_) => false,
        }
    }

    /// Make a wildcard variable with the given unique index.
    pub fn create_wildcard(index: usize) -> Variable {
        Variable::Universal(format!("{}{}", Self::WILDCARD_PREFIX, index).into())
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self {
            Variable::Universal(_) => "?",
            Variable::Existential(_) => "!",
        };

        write!(f, "{}{}", prefix, &self.name())
    }
}

/// A term with a specific constant value
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum Constant {
    /// An (abstract) constant.
    Abstract(Identifier),
    /// A numeric literal.
    NumericLiteral(NumericLiteral),
    /// A string literal.
    StringLiteral(String),
    /// An RDF-literal.
    RdfLiteral(RdfLiteral),
    /// A map literal.
    MapLiteral(Map),
    /// A tuple literal
    TupleLiteral(Tuple),
}

impl Constant {
    /// If this is a string literal, return the string.
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::StringLiteral(string) => Some(string),
            _ => None,
        }
    }

    /// If this is a constant, return the [Identifier].
    pub fn as_abstract(&self) -> Option<&Identifier> {
        match self {
            Self::Abstract(identifier) => Some(identifier),
            _ => None,
        }
    }

    /// If this is a resource (either a string literal, or a constant identifier), return it.
    pub fn as_resource(&self) -> Option<&nemo_physical::resource::Resource> {
        match self {
            Self::StringLiteral(string) => Some(string),
            Self::Abstract(identifier) => Some(&identifier.0),
            _ => None,
        }
    }

    /// Translate [Constant] into [AnyDataValue],
    /// ignoring type information
    /// TODO: Refactor rule model and get rid of this
    pub fn as_datavalue(&self) -> AnyDataValue {
        match self {
            Constant::Abstract(constant) => AnyDataValue::new_iri(constant.to_string()),
            Constant::NumericLiteral(constant) => match constant {
                NumericLiteral::Integer(integer) => AnyDataValue::new_integer_from_i64(*integer),
                NumericLiteral::Decimal(_integer, _fraction) => todo!(),
                NumericLiteral::Double(double) => {
                    AnyDataValue::new_double_from_f64(f64::from(*double))
                        .expect("We don't parse infinity or NaN")
                }
            },
            Constant::StringLiteral(string) => AnyDataValue::new_string(string.clone()),
            Constant::RdfLiteral(literal) => {
                match literal {
                    RdfLiteral::LanguageString { value, tag } => {
                        AnyDataValue::new_language_tagged_string(value.clone(), tag.clone())
                    }
                    RdfLiteral::DatatypeValue { value, datatype } => {
                        // TODO: Handle error
                        AnyDataValue::new_from_typed_literal(value.clone(), datatype.clone())
                            .expect("TOOD: Handle RDFLiteral Error")
                    }
                }
            }
            Constant::MapLiteral(_) => todo!(),
            Constant::TupleLiteral(_) => todo!(),
        }
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Abstract(Identifier(abstract_string)) => {
                // Nulls on logical level start with __Null# and shall be wrapped in angle brackets
                // blank nodes and anything that starts with an ascii letter (like bare names)
                // should not be wrapped in angle brackets
                if !abstract_string.starts_with(LOGICAL_NULL_PREFIX)
                    && abstract_string.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                {
                    write!(f, "{abstract_string}")
                }
                // everything else (including nulls) shall be wrapped in angle_brackets
                else {
                    write!(f, "<{abstract_string}>")
                }
            }
            Constant::NumericLiteral(literal) => write!(f, "{}", literal),
            Constant::StringLiteral(literal) => write!(f, "\"{}\"", literal),
            Constant::RdfLiteral(literal) => write!(f, "{}", literal),
            Constant::MapLiteral(term) => write!(f, "{term}"),
            Constant::TupleLiteral(tuple) => write!(f, "{tuple}"),
        }
    }
}

/// Simple term that is either a constant or a variable
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum PrimitiveTerm {
    /// A constant.
    Constant(Constant),
    /// A variable.
    Variable(Variable),
}

impl From<Constant> for PrimitiveTerm {
    fn from(value: Constant) -> Self {
        Self::Constant(value)
    }
}

impl PrimitiveTerm {
    /// Return `true` if term is not a variable.
    pub fn is_ground(&self) -> bool {
        !matches!(self, PrimitiveTerm::Variable(_))
    }
}

impl Display for PrimitiveTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveTerm::Constant(term) => write!(f, "{}", term),
            PrimitiveTerm::Variable(term) => write!(f, "{}", term),
        }
    }
}

/// Binary operation between two [`Term`]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum BinaryOperation {
    /// Sum between terms.
    Addition,
    /// Difference between two terms.
    Subtraction,
    /// Product between terms.
    Multiplication,
    /// Ratio between the two terms.
    Division,
    /// First term raised to the power of the second term.
    Exponent,
}

impl BinaryOperation {
    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            BinaryOperation::Addition => "Addition",
            BinaryOperation::Subtraction => "Subtraction",
            BinaryOperation::Multiplication => "Multiplication",
            BinaryOperation::Division => "Division",
            BinaryOperation::Exponent => "Exponent",
        };

        String::from(name)
    }
}

/// Unary operation applied to a [`Term`]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum UnaryOperation {
    /// Squareroot of the given term
    SquareRoot,
    /// Additive inverse of the given term.
    UnaryMinus,
    /// Absolute value of the given term.
    Abs,
}

impl UnaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a know unary function.
    pub fn construct_from_name(name: &str) -> Result<UnaryOperation, Error> {
        match name {
            "Abs" => Ok(UnaryOperation::Abs),
            "Sqrt" => Ok(UnaryOperation::SquareRoot),
            s => Err(Error::UnknonwUnaryOpertation {
                operation: s.into(),
            }),
        }
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            UnaryOperation::SquareRoot => "SquareRoot",
            UnaryOperation::UnaryMinus => "UnaryMinus",
            UnaryOperation::Abs => "Abs",
        };

        String::from(name)
    }
}

/// Possibly complex term that may occur within an [`super::Atom`]
#[derive(Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum Term {
    /// Primitive term.
    Primitive(PrimitiveTerm),
    /// Binary operation.
    Binary {
        /// The operation to be executed.
        operation: BinaryOperation,
        /// The left hand side operand.
        lhs: Box<Term>,
        /// The right hand side operand.
        rhs: Box<Term>,
    },
    /// Unary operation.
    Unary(UnaryOperation, Box<Term>),
    /// Aggregation.
    Aggregation(Aggregate),
    /// Abstract Function.
    Function(Identifier, Vec<Term>),
}

impl Term {
    /// If the term is a simple [`PrimitiveTerm`] then return it.
    /// Otherwise return `None`.
    pub(crate) fn as_primitive(&self) -> Option<PrimitiveTerm> {
        match self {
            Term::Primitive(primitive) => Some(primitive.clone()),
            _ => None,
        }
    }

    /// Returns `true` if term is primitive.
    /// Returns `false` if term is composite.
    pub(crate) fn is_primitive(&self) -> bool {
        self.as_primitive().is_some()
    }

    /// Return all [`PrimitiveTerm`]s that make up this term.
    pub(crate) fn primitive_terms(&self) -> Vec<&PrimitiveTerm> {
        match self {
            Term::Primitive(primitive) => {
                vec![primitive]
            }
            Term::Binary { lhs, rhs, .. } => {
                let mut terms = lhs.primitive_terms();
                terms.extend(rhs.primitive_terms());

                terms
            }
            Term::Unary(_, inner) => inner.primitive_terms(),
            Term::Function(_, subterms) => {
                subterms.iter().flat_map(|t| t.primitive_terms()).collect()
            }
            Term::Aggregation(aggregate) => aggregate.terms.iter().collect(),
        }
    }

    /// Return all variables in the term.
    pub(crate) fn variables(&self) -> impl Iterator<Item = &Variable> {
        self.primitive_terms()
            .into_iter()
            .filter_map(|term| match term {
                PrimitiveTerm::Variable(var) => Some(var),
                _ => None,
            })
    }

    /// Return all universally quantified variables in the term.
    pub(crate) fn universal_variables(&self) -> impl Iterator<Item = &Variable> {
        self.variables()
            .filter(|var| matches!(var, Variable::Universal(_)))
    }

    /// Return all existentially quantified variables in the term.
    pub(crate) fn existential_variables(&self) -> impl Iterator<Item = &Variable> {
        self.variables()
            .filter(|var| matches!(var, Variable::Existential(_)))
    }

    /// Replaces [`Variable`]s with [`Term`]s according to the provided assignment.
    pub(crate) fn apply_assignment(&mut self, assignment: &VariableAssignment) {
        match self {
            Term::Primitive(primitive) => {
                if let PrimitiveTerm::Variable(variable) = primitive {
                    if let Some(value) = assignment.get(variable) {
                        *self = value.clone();
                    }
                }
            }
            Term::Binary { lhs, rhs, .. } => {
                lhs.apply_assignment(assignment);
                rhs.apply_assignment(assignment);
            }
            Term::Unary(_, inner) => inner.apply_assignment(assignment),
            Term::Aggregation(aggregate) => aggregate.apply_assignment(assignment),
            Term::Function(_, subterms) => subterms
                .iter_mut()
                .for_each(|t| t.apply_assignment(assignment)),
        }
    }

    fn subterms_mut(&mut self) -> Vec<&mut Term> {
        match self {
            Term::Primitive(_primitive) => Vec::new(),
            Term::Binary {
                ref mut lhs,
                ref mut rhs,
                ..
            } => {
                vec![lhs, rhs]
            }
            Term::Unary(_, ref mut inner) => vec![inner],
            Term::Aggregation(_aggregate) => Vec::new(),
            Term::Function(_, subterms) => subterms.iter_mut().collect(),
        }
    }

    /// Mutate the term in place, calling the function `f` on itself and recursively on it's subterms if the function `f` returns true
    ///
    /// This is used e.g. to rewrite aggregates inside of constructors with placeholder variables
    pub(crate) fn update_subterms_recursively<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut Term) -> bool,
    {
        f(self);

        for subterm in self.subterms_mut() {
            let should_recurse = f(subterm);

            if should_recurse {
                subterm.update_subterms_recursively(f);
            }
        }
    }

    // TODO: Was needed for tracing which will be reimplemented after the refactoring
    // /// Evaluates a constant (numeric) term.
    // pub fn evaluate_constant_numeric(
    //     &self,
    //     ty: &PrimitiveType,
    //     dict: &Dict,
    // ) -> Option<StorageValueT> {
    //     let arithmetic_tree = compile_termtree(self, &VariableOrder::new(), ty);
    //     let storage_type = ty.datatype_name().to_storage_type_name();

    //     macro_rules! translate_data_type {
    //         ($variant:ident, $type:ty) => {{
    //             let translate_function = |l: &StackValue<DataValueT>| match l {
    //                 StackValue::Constant(t) => {
    //                     if let StorageValueT::$variant(value) = t
    //                         .to_storage_value(dict)
    //                         .expect("We expect all strings to be known at this point.")
    //                     {
    //                         StackValue::Constant(value)
    //                     } else {
    //                         panic!(
    //                             "Expected a operation tree value of type {}",
    //                             stringify!($src_name)
    //                         );
    //                     }
    //                 }
    //                 StackValue::Reference(index) => StackValue::Reference(*index),
    //             };

    //             let arithmetic_tree_typed = arithmetic_tree.map_values(&translate_function);
    //             Some(StorageValueT::$variant(
    //                 arithmetic_tree_typed.evaluate(&mut Vec::new(), &[])?,
    //             ))
    //         }};
    //     }

    //     match storage_type {
    //         StorageTypeName::Id32 => translate_data_type!(Id32, u32),
    //         StorageTypeName::Id64 => translate_data_type!(Id64, u64),
    //         StorageTypeName::Int64 => translate_data_type!(Int64, i64),
    //         StorageTypeName::Float => translate_data_type!(Float, f32),
    //         StorageTypeName::Double => translate_data_type!(Double, f64),
    //     }
    // }
}

impl From<PrimitiveTerm> for Term {
    fn from(value: PrimitiveTerm) -> Self {
        Term::Primitive(value)
    }
}

impl Term {
    fn ascii_tree(&self) -> ascii_tree::Tree {
        match self {
            Term::Primitive(primitive) => ascii_tree::Tree::Leaf(vec![format!("{:?}", primitive)]),
            Term::Binary {
                operation,
                lhs,
                rhs,
            } => ascii_tree::Tree::Node(operation.name(), vec![lhs.ascii_tree(), rhs.ascii_tree()]),
            Term::Unary(operation, inner) => {
                ascii_tree::Tree::Node(operation.name(), vec![inner.ascii_tree()])
            }
            Term::Aggregation(aggregate) => {
                ascii_tree::Tree::Leaf(vec![format!("{:?}", aggregate)])
            }
            Term::Function(function, subterms) => ascii_tree::Tree::Node(
                function.to_string(),
                subterms.iter().map(|s| s.ascii_tree()).collect(),
            ),
        }
    }

    /// Defines the precedence of the term operations.
    /// This is only relevant for the [`Display`] implementation.
    fn precedence(&self) -> usize {
        match self {
            Term::Primitive(_) => 0,
            Term::Binary {
                operation: BinaryOperation::Addition,
                ..
            } => 1,
            Term::Binary {
                operation: BinaryOperation::Subtraction,
                ..
            } => 1,
            Term::Binary {
                operation: BinaryOperation::Multiplication,
                ..
            } => 2,
            Term::Binary {
                operation: BinaryOperation::Division,
                ..
            } => 2,
            Term::Binary {
                operation: BinaryOperation::Exponent,
                ..
            } => 3,
            Term::Unary(_, _) => 5,
            Term::Aggregation(_) => 5,
            Term::Function(_, _) => 5,
        }
    }

    fn format_braces_priority(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        term: &Term,
    ) -> std::fmt::Result {
        let need_braces = self.precedence() > term.precedence() && !term.is_primitive();

        if need_braces {
            self.format_braces(f, term)
        } else {
            write!(f, "{}", term)
        }
    }

    fn format_braces(&self, f: &mut std::fmt::Formatter<'_>, term: &Term) -> std::fmt::Result {
        f.write_str("(")?;
        write!(f, "{}", term)?;
        f.write_str(")")
    }

    fn format_multinary_operation(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        terms: &[Term],
        delimiter: &str,
    ) -> std::fmt::Result {
        for (index, term) in terms.iter().enumerate() {
            self.format_braces_priority(f, term)?;

            if index < terms.len() - 1 {
                f.write_str(delimiter)?;
            }
        }

        Ok(())
    }

    fn format_binary_operation(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        left: &Term,
        right: &Term,
        operation: BinaryOperation,
    ) -> std::fmt::Result {
        self.format_braces_priority(f, left)?;
        match operation {
            BinaryOperation::Addition => write!(f, " + ")?,
            BinaryOperation::Subtraction => write!(f, " - ")?,
            BinaryOperation::Multiplication => write!(f, " * ")?,
            BinaryOperation::Division => write!(f, " / ")?,
            BinaryOperation::Exponent => write!(f, " ^ ")?,
        }
        self.format_braces_priority(f, right)
    }
}

impl Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        ascii_tree::write_tree(f, &self.ascii_tree())
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Primitive(primitive) => write!(f, "{}", primitive),
            Term::Binary {
                operation,
                lhs,
                rhs,
            } => self.format_binary_operation(f, lhs, rhs, *operation),
            Term::Unary(UnaryOperation::SquareRoot, inner) => {
                write!(f, "sqrt({})", inner)
            }
            Term::Unary(UnaryOperation::UnaryMinus, inner) => {
                write!(f, "-")?;
                self.format_braces_priority(f, inner)
            }
            Term::Unary(UnaryOperation::Abs, inner) => {
                write!(f, "|{}|", inner)
            }
            Term::Aggregation(aggregate) => write!(f, "{}", aggregate),
            Term::Function(function, subterms) => {
                f.write_str(&function.to_string())?;
                f.write_str("(")?;
                self.format_multinary_operation(f, subterms, ", ")?;
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::assert_eq;

    use nemo_physical::datatypes::Double;

    use crate::model::{
        InvalidRdfLiteral, NumericLiteral, RdfLiteral, XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER,
        XSD_STRING,
    };

    use super::*;

    #[test]
    fn rdf_literal_normalization() {
        let language_string_literal = RdfLiteral::LanguageString {
            value: "language string".to_string(),
            tag: "en".to_string(),
        };
        let random_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "some random datavalue".to_string(),
            datatype: "a datatype that I totally did not just make up".to_string(),
        };
        let string_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "string datavalue".to_string(),
            datatype: XSD_STRING.to_string(),
        };
        let integer_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "73".to_string(),
            datatype: XSD_INTEGER.to_string(),
        };
        let decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "1.23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let signed_decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "+1.23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let negative_decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "-1.23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let pointless_decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let signed_pointless_decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "+23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let negative_pointless_decimal_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "-23".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let double_datavalue_literal = RdfLiteral::DatatypeValue {
            value: "3.33".to_string(),
            datatype: XSD_DOUBLE.to_string(),
        };
        let large_integer_literal = RdfLiteral::DatatypeValue {
            value: "9950000000000000000".to_string(),
            datatype: XSD_INTEGER.to_string(),
        };
        let large_decimal_literal = RdfLiteral::DatatypeValue {
            value: "9950000000000000001".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };
        let invalid_integer_literal = RdfLiteral::DatatypeValue {
            value: "123.45".to_string(),
            datatype: XSD_INTEGER.to_string(),
        };
        let invalid_decimal_literal = RdfLiteral::DatatypeValue {
            value: "123.45a".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        };

        let expected_language_string_literal =
            Constant::RdfLiteral(language_string_literal.clone());
        let expected_random_datavalue_literal =
            Constant::RdfLiteral(random_datavalue_literal.clone());
        let expected_string_datavalue_literal =
            Constant::StringLiteral("string datavalue".to_string());
        let expected_integer_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Integer(73));
        let expected_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(1, 23));
        let expected_signed_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(1, 23));
        let expected_negative_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(-1, 23));
        let expected_pointless_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(23, 0));
        let expected_signed_pointless_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(23, 0));
        let expected_negative_pointless_decimal_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Decimal(-23, 0));
        let expected_double_datavalue_literal =
            Constant::NumericLiteral(NumericLiteral::Double(Double::new(3.33).unwrap()));
        let expected_large_integer_literal = Constant::RdfLiteral(RdfLiteral::DatatypeValue {
            value: "9950000000000000000".to_string(),
            datatype: XSD_INTEGER.to_string(),
        });
        let expected_large_decimal_literal = Constant::RdfLiteral(RdfLiteral::DatatypeValue {
            value: "9950000000000000001".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        });
        let expected_invalid_integer_literal = InvalidRdfLiteral::new(RdfLiteral::DatatypeValue {
            value: "123.45".to_string(),
            datatype: XSD_INTEGER.to_string(),
        });
        let expected_invalid_decimal_literal = InvalidRdfLiteral::new(RdfLiteral::DatatypeValue {
            value: "123.45a".to_string(),
            datatype: XSD_DECIMAL.to_string(),
        });

        assert_eq!(
            Constant::try_from(language_string_literal).unwrap(),
            expected_language_string_literal
        );
        assert_eq!(
            Constant::try_from(random_datavalue_literal).unwrap(),
            expected_random_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(string_datavalue_literal).unwrap(),
            expected_string_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(integer_datavalue_literal).unwrap(),
            expected_integer_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(decimal_datavalue_literal).unwrap(),
            expected_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(signed_decimal_datavalue_literal).unwrap(),
            expected_signed_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(negative_decimal_datavalue_literal).unwrap(),
            expected_negative_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(pointless_decimal_datavalue_literal).unwrap(),
            expected_pointless_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(signed_pointless_decimal_datavalue_literal).unwrap(),
            expected_signed_pointless_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(negative_pointless_decimal_datavalue_literal).unwrap(),
            expected_negative_pointless_decimal_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(double_datavalue_literal).unwrap(),
            expected_double_datavalue_literal
        );
        assert_eq!(
            Constant::try_from(large_integer_literal).unwrap(),
            expected_large_integer_literal
        );
        assert_eq!(
            Constant::try_from(large_decimal_literal).unwrap(),
            expected_large_decimal_literal
        );
        assert_eq!(
            Constant::try_from(invalid_integer_literal).unwrap_err(),
            expected_invalid_integer_literal
        );
        assert_eq!(
            Constant::try_from(invalid_decimal_literal).unwrap_err(),
            expected_invalid_decimal_literal
        );
    }
}
