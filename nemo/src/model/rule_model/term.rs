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
    GroundTerm(AnyDataValue),
    /// A variable.
    Variable(Variable),
}

impl From<AnyDataValue> for PrimitiveTerm {
    fn from(value: AnyDataValue) -> Self {
        Self::GroundTerm(value)
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
            PrimitiveTerm::GroundTerm(term) => write!(f, "{}", term),
            PrimitiveTerm::Variable(term) => write!(f, "{}", term),
        }
    }
}

/// Binary operation between two [Term]s.
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
    /// Remainder of a division between two numeric values
    NumericRemainder,
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
    /// Check whether string is contained in another, correspondng to SPARQL function CONTAINS.
    StringContains,
    /// String starting at some start position
    StringSubstring,
    /// First part of a string split by some other string
    StringBefore,
    /// Second part of a string split by some other string
    StringAfter,
    /// Whether string starts with a certain string
    StringStarts,
    /// Whether string ends with a certain string
    StringEnds,
}

impl BinaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a known binary function.
    pub fn construct_from_name(name: &str) -> Option<BinaryOperation> {
        Some(match name.to_uppercase().as_str() {
            "LOG" => Self::NumericLogarithm,
            "POW" => Self::NumericPower,
            "COMPARE" => Self::StringCompare,
            "CONTAINS" => Self::StringContains,
            "SUBSTR" => Self::StringSubstring,
            "STRSTARTS" => Self::StringStarts,
            "STRENDS" => Self::StringEnds,
            "STRBEFORE" => Self::StringBefore,
            "STRAFTER" => Self::StringAfter,
            "REM" => Self::NumericRemainder,
            _ => return None,
        })
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            Self::NumericAddition => "Addition",
            Self::NumericSubtraction => "Subtraction",
            Self::NumericMultiplication => "Multiplication",
            Self::NumericDivision => "Division",
            Self::NumericPower => "POW",
            Self::NumericRemainder => "Remainder",
            Self::NumericLogarithm => "Logarithm",
            Self::StringCompare => "StringCompare",
            Self::StringContains => "CONTAINS",
            Self::StringSubstring => "SUBSTR",
            Self::Equal => "Equals",
            Self::Unequals => "Unequals",
            Self::NumericGreaterthan => "GreaterThan",
            Self::NumericGreaterthaneq => "GreaterThanEq",
            Self::NumericLessthan => "LessThan",
            Self::NumericLessthaneq => "LessThanEq",
            Self::StringBefore => "STRBEFORE",
            Self::StringAfter => "STRAFTER",
            Self::StringStarts => "STRSTARTS",
            Self::StringEnds => "STRENDS",
        };

        String::from(name)
    }

    /// Return the infix operator for this operation
    /// or `None` if this is not an infix operation
    pub fn infix(&self) -> Option<&'static str> {
        match self {
            Self::NumericAddition => Some("+"),
            Self::NumericSubtraction => Some("-"),
            Self::NumericMultiplication => Some("*"),
            Self::NumericDivision => Some("/"),
            Self::Equal => Some("="),
            Self::Unequals => Some("!="),
            Self::NumericGreaterthan => Some(">"),
            Self::NumericGreaterthaneq => Some(">="),
            Self::NumericLessthan => Some("<"),
            Self::NumericLessthaneq => Some("<="),
            Self::NumericRemainder => Some("%"),
            Self::NumericLogarithm
            | Self::NumericPower
            | Self::StringCompare
            | Self::StringContains
            | Self::StringSubstring
            | Self::StringStarts
            | Self::StringEnds
            | Self::StringBefore
            | Self::StringAfter => None,
        }
    }
}

/// Ternary operation applied to a [Term]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum TernaryOperation {
    /// String starting at some start position with a given length
    StringSubstringLength,
}

impl TernaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a known binary function.
    pub fn construct_from_name(name: &str) -> Option<TernaryOperation> {
        Some(match name.to_uppercase().as_str() {
            "SUBSTRING" => Self::StringSubstringLength,
            _ => return None,
        })
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            TernaryOperation::StringSubstringLength => "SUBSTRING",
        };

        String::from(name)
    }
}

/// N-ary operation applied to a [Term]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum NaryOperation {
    /// Bitwise and operation
    BitAnd,
    /// Bitwise or operation
    BitOr,
    /// Bitwise xor operation
    BitXor,
    /// Conjunction of boolean values
    BooleanConjunction,
    /// Disjunction of boolean values
    BooleanDisjunction,
    /// Sum of numeric values
    NumericSum,
    /// Product of numeric values
    NumericProduct,
    /// Minimum of numeric values
    NumericMinimum,
    /// Maximum of numeric values
    NumericMaximum,
    /// Lukasiewicz norm of numeric values
    NumericLukasiewicz,
    /// Concatentation of two string values, correspondng to SPARQL function CONCAT.
    StringConcatenation,
}

impl NaryOperation {
    /// Return a function which is able to construct the respective term based on the function name.
    /// Returns `None` if the provided function name does not correspond to a known binary function.
    pub fn construct_from_name(name: &str) -> Option<NaryOperation> {
        Some(match name.to_uppercase().as_str() {
            "BITAND" => Self::BitAnd,
            "BITOR" => Self::BitOr,
            "BITXOR" => Self::BitXor,
            "MAX" => Self::NumericMaximum,
            "MIN" => Self::NumericMinimum,
            "LUKA" => Self::NumericLukasiewicz,
            "SUM" => Self::NumericSum,
            "PROD" => Self::NumericProduct,
            "AND" => Self::BooleanConjunction,
            "OR" => Self::BooleanDisjunction,
            "CONCAT" => Self::StringConcatenation,
            _ => return None,
        })
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            Self::StringConcatenation => "CONCAT",
            Self::BooleanConjunction => "AND",
            Self::BooleanDisjunction => "OR",
            Self::BitAnd => "BITAND",
            Self::BitOr => "BITOR",
            Self::BitXor => "BITXOR",
            Self::NumericSum => "SUM",
            Self::NumericProduct => "PROD",
            Self::NumericMinimum => "MIN",
            Self::NumericMaximum => "MAX",
            Self::NumericLukasiewicz => "LUKA",
        };

        String::from(name)
    }
}

/// Unary operation applied to a [Term]
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd, Ord)]
pub enum UnaryOperation {
    /// Boolean negation
    BooleanNegation,
    /// Cast to double
    CastToDouble,
    /// Cast to float
    CastToFloat,
    /// Cast to integer
    CastToInteger,
    /// Canonical string representation of a value
    CanonicalString,
    /// Check if value is an integer
    CheckIsInteger,
    /// Check if value is a float
    CheckIsFloat,
    /// Check if value is a double
    CheckIsDouble,
    /// Check if value is an iri
    CheckIsIri,
    /// Check if value is numeric
    CheckIsNumeric,
    /// Check if value is a null
    CheckIsNull,
    /// Check if value is a string
    CheckIsString,
    /// Get datatype of a value
    Datatype,
    /// Get language tag of a languaged tagged string
    LanguageTag,
    /// Lexical value
    LexicalValue,
    /// Absolute value of a numeric value
    NumericAbsolute,
    /// Cosine of a numeric value
    NumericCosine,
    /// Rounding up of a numeric value
    NumericCeil,
    /// Rounding down of a numeric value
    NumericFloor,
    /// Additive inverse of a numeric value
    NumericNegation,
    /// Rounding of a numeric value
    NumericRound,
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
            "isInteger" => Ok(UnaryOperation::CheckIsInteger),
            "isFloat" => Ok(UnaryOperation::CheckIsFloat),
            "isDouble" => Ok(UnaryOperation::CheckIsDouble),
            "isIri" => Ok(UnaryOperation::CheckIsIri),
            "isNumeric" => Ok(UnaryOperation::CheckIsNumeric),
            "isNull" => Ok(UnaryOperation::CheckIsNull),
            "isString" => Ok(UnaryOperation::CheckIsString),
            "ABS" => Ok(UnaryOperation::NumericAbsolute),
            "SQRT" => Ok(UnaryOperation::NumericSquareroot),
            "NOT" => Ok(UnaryOperation::BooleanNegation),
            "fullStr" => Ok(UnaryOperation::CanonicalString),
            "STR" => Ok(UnaryOperation::LexicalValue),
            "SIN" => Ok(UnaryOperation::NumericSine),
            "COS" => Ok(UnaryOperation::NumericCosine),
            "TAN" => Ok(UnaryOperation::NumericTangent),
            "STRLEN" => Ok(UnaryOperation::StringLength),
            "UCASE" => Ok(UnaryOperation::StringLowercase),
            "LCASE" => Ok(UnaryOperation::StringUppercase),
            "ROUND" => Ok(UnaryOperation::NumericRound),
            "CEIL" => Ok(UnaryOperation::NumericCeil),
            "FLOOR" => Ok(UnaryOperation::NumericFloor),
            "DATATYPE" => Ok(UnaryOperation::Datatype),
            "LANG" => Ok(UnaryOperation::LanguageTag),
            "INT" => Ok(UnaryOperation::CastToInteger),
            "DOUBLE" => Ok(UnaryOperation::CastToDouble),
            "FLOAT" => Ok(UnaryOperation::CastToFloat),
            s => Err(Error::UnknownUnaryOpertation {
                operation: s.into(),
            }),
        }
    }

    /// Return the name of the operation.
    pub fn name(&self) -> String {
        let name = match self {
            Self::NumericSquareroot => "SQRT",
            Self::NumericNegation => "MINUS",
            Self::NumericAbsolute => "ABS",
            Self::BooleanNegation => "NOT",
            Self::CanonicalString => "fullStr",
            Self::NumericCosine => "COS",
            Self::NumericSine => "SIN",
            Self::NumericTangent => "TAN",
            Self::StringLength => "STRLEN",
            Self::StringLowercase => "LCASE",
            Self::StringUppercase => "UCASE",
            Self::NumericCeil => "CEIL",
            Self::NumericFloor => "FLOOR",
            Self::NumericRound => "ROUND",
            Self::CastToInteger => "INT",
            Self::CastToDouble => "DOUBLE",
            Self::CastToFloat => "FLOAT",
            Self::CheckIsInteger => "isInteger",
            Self::CheckIsFloat => "isFloat",
            Self::CheckIsDouble => "isDouble",
            Self::CheckIsIri => "isIri",
            Self::CheckIsNumeric => "IsNumeric",
            Self::CheckIsNull => "isNull",
            Self::CheckIsString => "isString",
            Self::Datatype => "DATATYPE",
            Self::LanguageTag => "LANG",
            Self::LexicalValue => "STR",
        };

        String::from(name)
    }
}

/// Possibly complex term that may occur within an [super::Atom]
#[derive(Eq, PartialEq, Clone, PartialOrd, Ord)]
pub enum Term {
    /// Primitive term.
    Primitive(PrimitiveTerm),
    /// Unary operation.
    Unary(UnaryOperation, Box<Term>),
    /// Binary operation.
    Binary {
        /// The operation to be executed.
        operation: BinaryOperation,
        /// The left hand side operand.
        lhs: Box<Term>,
        /// The right hand side operand.
        rhs: Box<Term>,
    },
    /// Ternary operation.
    Ternary {
        /// The operation to be executed.
        operation: TernaryOperation,
        /// The first operand.
        first: Box<Term>,
        /// The second operand.
        second: Box<Term>,
        /// The third operand.
        third: Box<Term>,
    },
    /// An n-ary operation.
    Nary {
        /// The operation to be executed.
        operation: NaryOperation,
        /// Its parameters
        parameters: Vec<Term>,
    },
    /// Aggregation.
    Aggregation(Aggregate),
    /// Abstract Function.
    Function(Identifier, Vec<Term>),
}

impl Term {
    /// If the term is a simple [PrimitiveTerm] then return it.
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

    /// Return whether this term is a variable.
    pub(crate) fn is_variable(&self) -> bool {
        matches!(self, Term::Primitive(PrimitiveTerm::Variable(_)))
    }

    /// Return all [PrimitiveTerm]s that make up this term.
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
            Term::Ternary {
                first,
                second,
                third,
                ..
            } => {
                let mut terms = first.primitive_terms();
                terms.extend(second.primitive_terms());
                terms.extend(third.primitive_terms());

                terms
            }
            Term::Nary { parameters, .. } => parameters
                .iter()
                .flat_map(|p| p.primitive_terms())
                .collect(),
            Term::Unary(_, inner) => inner.primitive_terms(),
            Term::Function(_, subterms) => {
                subterms.iter().flat_map(|t| t.primitive_terms()).collect()
            }
            Term::Aggregation(aggregate) => aggregate
                .terms
                .iter()
                .flat_map(|term| term.primitive_terms())
                .collect(),
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
                PrimitiveTerm::GroundTerm(datavalue) => Some(datavalue),
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

    /// Replaces [Variable]s with [Term]s according to the provided assignment.
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
            Term::Ternary {
                first,
                second,
                third,
                ..
            } => {
                first.apply_assignment(assignment);
                second.apply_assignment(assignment);
                third.apply_assignment(assignment);
            }
            Term::Nary { parameters, .. } => {
                parameters
                    .iter_mut()
                    .for_each(|t| t.apply_assignment(assignment));
            }
        }
    }

    fn subterms_mut(&mut self) -> Vec<&mut Term> {
        match self {
            Term::Primitive(_primitive) => Vec::new(),
            Term::Unary(_, ref mut inner) => vec![inner],
            Term::Binary {
                ref mut lhs,
                ref mut rhs,
                ..
            } => {
                vec![lhs, rhs]
            }
            Term::Ternary {
                ref mut first,
                ref mut second,
                ref mut third,
                ..
            } => vec![first, second, third],
            Term::Nary {
                operation: _,
                parameters,
            } => parameters.iter_mut().collect(),
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

    /// Return all aggreagtes constained in this term.
    pub(crate) fn aggregates(&self) -> Vec<Aggregate> {
        match self {
            Term::Primitive(_) => vec![],
            Term::Unary(_, subterm) => subterm.aggregates(),
            Term::Binary {
                operation: _,
                lhs,
                rhs,
            } => {
                let mut result = lhs.aggregates();
                result.extend(rhs.aggregates());
                result
            }
            Term::Ternary {
                operation: _,
                first,
                second,
                third,
            } => {
                let mut result = first.aggregates();
                result.extend(second.aggregates());
                result.extend(third.aggregates());

                result
            }
            Term::Nary {
                operation: _,
                parameters,
            } => {
                let mut result = Vec::<Aggregate>::new();
                for subterm in parameters {
                    result.extend(subterm.aggregates());
                }
                result
            }
            Term::Aggregation(aggregate) => {
                let mut result = vec![aggregate.clone()];

                for subterm in &aggregate.terms {
                    result.extend(subterm.aggregates());
                }

                result
            }
            Term::Function(_, _) => panic!("Function symbols not supported"),
        }
    }
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
            Term::Ternary {
                operation,
                first,
                second,
                third,
            } => ascii_tree::Tree::Node(
                operation.name(),
                vec![first.ascii_tree(), second.ascii_tree(), third.ascii_tree()],
            ),
            Term::Nary {
                operation,
                parameters,
            } => ascii_tree::Tree::Node(
                operation.name(),
                parameters.iter().map(|p| p.ascii_tree()).collect(),
            ),
        }
    }

    /// Defines the precedence of the term operations.
    /// This is only relevant for the [Display] implementation.
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
                operation: BinaryOperation::NumericMultiplication,
                ..
            } => 2,
            Term::Binary {
                operation: BinaryOperation::NumericDivision,
                ..
            } => 2,
            Term::Binary { .. } => 3,
            Term::Ternary { .. } => 3,
            Term::Nary { .. } => 5,
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

    fn format_nary_operation(
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

    fn format_ternary_operation(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        first: &Term,
        second: &Term,
        third: &Term,
        operation: TernaryOperation,
    ) -> std::fmt::Result {
        write!(f, "{}({}, {}, {})", operation.name(), first, second, third)
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
                self.format_nary_operation(f, subterms, ", ")?;
                f.write_str(")")
            }
            Term::Ternary {
                operation,
                first,
                second,
                third,
            } => self.format_ternary_operation(f, first, second, third, *operation),
            Term::Nary {
                operation,
                parameters,
            } => {
                f.write_str(&operation.name())?;
                f.write_str("(")?;
                self.format_nary_operation(f, parameters, ", ")?;
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod test {}
