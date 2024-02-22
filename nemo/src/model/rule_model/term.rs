use std::fmt::{Debug, Display};

use nemo_physical::datavalues::AnyDataValue;

use crate::{error::Error, model::VariableAssignment};

use super::{Aggregate, Identifier};

/// Variable that can be bound to a specific value.
/// Variables are identified by a string name or (in the case of
/// invented variable names) by numeric ids.
#[derive(Debug, Eq, PartialEq, Hash, Clone, PartialOrd, Ord)]
pub enum Variable {
    /// A universally quantified variable.
    Universal(String),
    /// An existentially quantified variable.
    Existential(String),
    /// An unnamed variable identified by a numeric id.
    UnnamedUniversal(usize),
}

impl Variable {
    /// Return the string name of the variable, or `None` if
    /// the variable is unnamed.
    ///
    /// Note: Use `Display` or `Debug` for error messages etc.
    pub fn name(&self) -> Option<String> {
        match self {
            Self::Universal(identifier) | Self::Existential(identifier) => {
                Some(identifier.to_owned())
            }
            Self::UnnamedUniversal(_) => None,
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
    pub fn is_unnamed(&self) -> bool {
        match self {
            Self::Universal(_) | Self::Existential(_) => false,
            Self::UnnamedUniversal(_) => true,
        }
    }

    /// Make an unnamed variable with the given unique index.
    pub fn new_unamed(index: usize) -> Variable {
        Self::UnnamedUniversal(index)
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Universal(var_name) => write!(f, "?{}", var_name),
            Self::Existential(var_name) => write!(f, "!{}", var_name),
            Self::UnnamedUniversal(_) => write!(f, "_"),
        }
    }
}

/// Simple term that is either a constant or a variable
#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum PrimitiveTerm {
    /// A constant.
    Constant(AnyDataValue),
    /// A variable.
    Variable(Variable),
}

impl From<AnyDataValue> for PrimitiveTerm {
    fn from(value: AnyDataValue) -> Self {
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
    /// Equality
    Equal,
    /// Inequality
    Unequals,
    /// Addition between two numeric values
    NumericAddition,
    /// Subtraction between two numeric values
    NumericSubtraction,
    /// Multiplication between two numeric values
    NumericMultiplication,
    /// Division between two numeric values
    NumericDivision,
    /// Logarithm of a numeric value to some numeric base
    NumericLogarithm,
    /// Numeric value raised to another numeric value
    NumericPower,
    /// Numeric greater than comparison
    NumericGreaterthan,
    /// Numeric greater than or equals comparison
    NumericGreaterthaneq,
    /// Numeric less than comparison
    NumericLessthan,
    /// Numeric less than or equals comparison
    NumericLessthaneq,
    /// Lexicographic comparison between strings
    StringCompare,
    /// Concatentation of two string values
    StringConcatenation,
    /// Check whether string is contained in another
    StringContains,
    /// The first part (of specified length) of a string
    StringSubstring,
    /// Conjunction of boolean values
    BooleanConjunction,
    /// Disjunction of boolean values
    BooleanDisjunction,
}

impl BinaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a known binary function.
    pub fn construct_from_name(name: &str) -> Result<BinaryOperation, Error> {
        match name {
            "Log" => Ok(BinaryOperation::NumericLogarithm),
            "Pow" => Ok(BinaryOperation::NumericPower),
            "Compare" => Ok(BinaryOperation::StringCompare),
            "Concat" => Ok(BinaryOperation::StringConcatenation),
            "Contains" => Ok(BinaryOperation::StringContains),
            "Substr" => Ok(BinaryOperation::StringSubstring),
            "And" => Ok(BinaryOperation::BooleanConjunction),
            "Or" => Ok(BinaryOperation::BooleanDisjunction),
            s => Err(Error::UnknownUnaryOpertation {
                operation: s.into(),
            }),
        }
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            BinaryOperation::NumericAddition => "Addition",
            BinaryOperation::NumericSubtraction => "Subtraction",
            BinaryOperation::NumericMultiplication => "Multiplication",
            BinaryOperation::NumericDivision => "Division",
            BinaryOperation::NumericPower => "Power",
            BinaryOperation::NumericLogarithm => "Logarithm",
            BinaryOperation::StringCompare => "StirngCompare",
            BinaryOperation::StringConcatenation => "StringConcatenation",
            BinaryOperation::StringContains => "StringContains",
            BinaryOperation::StringSubstring => "Substring",
            BinaryOperation::BooleanConjunction => "BooleanAnd",
            BinaryOperation::BooleanDisjunction => "BooleanOr",
            BinaryOperation::Equal => "Equals",
            BinaryOperation::Unequals => "Unequals",
            BinaryOperation::NumericGreaterthan => "GreaterThan",
            BinaryOperation::NumericGreaterthaneq => "GreaterThanEq",
            BinaryOperation::NumericLessthan => "LessThan",
            BinaryOperation::NumericLessthaneq => "LessThanEq",
        };

        String::from(name)
    }

    /// Return the infix operator for this operation
    /// or `None` if this is not an infix operation
    pub fn infix(&self) -> Option<&'static str> {
        match self {
            BinaryOperation::NumericAddition => Some("+"),
            BinaryOperation::NumericSubtraction => Some("-"),
            BinaryOperation::NumericMultiplication => Some("*"),
            BinaryOperation::NumericDivision => Some("/"),
            BinaryOperation::Equal => Some("="),
            BinaryOperation::Unequals => Some("!="),
            BinaryOperation::NumericGreaterthan => Some(">"),
            BinaryOperation::NumericGreaterthaneq => Some(">="),
            BinaryOperation::NumericLessthan => Some("<"),
            BinaryOperation::NumericLessthaneq => Some("<="),
            BinaryOperation::NumericLogarithm
            | BinaryOperation::NumericPower
            | BinaryOperation::StringCompare
            | BinaryOperation::StringConcatenation
            | BinaryOperation::StringContains
            | BinaryOperation::StringSubstring
            | BinaryOperation::BooleanConjunction
            | BinaryOperation::BooleanDisjunction => None,
        }
    }
}

/// Unary operation applied to a [`Term`]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum UnaryOperation {
    /// Boolean negation
    BooleanNegation,
    /// Canonical string representation of a value
    CanonicalString,
    /// Absolute value of a numeric value
    NumericAbsolute,
    /// Cosine of a numeric value
    NumericCosine,
    /// Additive inverse of a numeric value
    NumericNegation,
    /// Sine of a numeric value
    NumericSine,
    /// Square root of a numeric value
    NumericSquareroot,
    /// Tangent of a numeric value
    NumericTangent,
    /// Length of a string value
    StringLength,
    /// String converted to lowercase letters
    StringLowercase,
    /// String converted to uppercase letters
    StringUppercase,
}

impl UnaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a know unary function.
    pub fn construct_from_name(name: &str) -> Result<UnaryOperation, Error> {
        match name {
            "Abs" => Ok(UnaryOperation::NumericAbsolute),
            "Sqrt" => Ok(UnaryOperation::NumericSquareroot),
            "Neg" => Ok(UnaryOperation::BooleanNegation),
            "String" => Ok(UnaryOperation::CanonicalString),
            "Sin" => Ok(UnaryOperation::NumericSine),
            "Cos" => Ok(UnaryOperation::NumericCosine),
            "Tan" => Ok(UnaryOperation::NumericTangent),
            "Length" => Ok(UnaryOperation::StringLength),
            "Lower" => Ok(UnaryOperation::StringLowercase),
            "Upper" => Ok(UnaryOperation::StringUppercase),
            s => Err(Error::UnknownUnaryOpertation {
                operation: s.into(),
            }),
        }
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            UnaryOperation::NumericSquareroot => "SquareRoot",
            UnaryOperation::NumericNegation => "UnaryMinus",
            UnaryOperation::NumericAbsolute => "Abs",
            UnaryOperation::BooleanNegation => "BooleanNegation",
            UnaryOperation::CanonicalString => "CanonicalString",
            UnaryOperation::NumericCosine => "Cosine",
            UnaryOperation::NumericSine => "Sine",
            UnaryOperation::NumericTangent => "Tangent",
            UnaryOperation::StringLength => "StringLength",
            UnaryOperation::StringLowercase => "Lowercase",
            UnaryOperation::StringUppercase => "Uppercase",
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

    /// Return all [AnyDataValue]s that appear in the term.
    pub(crate) fn datavalues(&self) -> impl Iterator<Item = &AnyDataValue> {
        self.primitive_terms()
            .into_iter()
            .filter_map(|term| match term {
                PrimitiveTerm::Constant(datavalue) => Some(datavalue),
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
                operation: BinaryOperation::NumericAddition,
                ..
            } => 1,
            Term::Binary {
                operation: BinaryOperation::NumericSubtraction,
                ..
            } => 1,
            Term::Binary {
                operation: BinaryOperation::NumericSubtraction,
                ..
            } => 2,
            Term::Binary {
                operation: BinaryOperation::NumericDivision,
                ..
            } => 2,
            Term::Binary { .. } => 3,
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
        if let Some(operator) = operation.infix() {
            self.format_braces_priority(f, left)?;

            write!(f, " {operator} ")?;

            self.format_braces_priority(f, right)
        } else {
            write!(f, "{}({}, {})", operation.name(), left, right)
        }
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
            Term::Unary(UnaryOperation::NumericNegation, inner) => {
                write!(f, "-")?;
                self.format_braces_priority(f, inner)
            }
            Term::Unary(UnaryOperation::NumericAbsolute, inner) => {
                write!(f, "|{}|", inner)
            }
            Term::Unary(operation, inner) => {
                write!(f, "{}({})", operation.name(), inner)
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
mod test {}
