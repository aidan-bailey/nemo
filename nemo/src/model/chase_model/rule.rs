//! Defines a variant of [crate::model::Rule], suitable for computing the chase.

use std::collections::{HashMap, HashSet};

use nemo_physical::datavalues::AnyDataValue;

use crate::{
    error::Error,
    model::{
        chase_model::variable::{AGGREGATE_VARIABLE_PREFIX, CONSTRUCT_VARIABLE_PREFIX},
        Aggregate, Constraint, Literal, PrimitiveTerm, Rule, Term, Variable,
    },
};

use super::{
    variable::EQUALITY_VARIABLE_PREFIX, ChaseAggregate, ChaseAtom, Constructor, PrimitiveAtom,
    VariableAtom,
};

/// Representation of a rule in a [super::ChaseProgram].
///
/// Chase rules may include placeholder variables, which start with `_`
/// * Additional constraints: `_EQUALITY_{term_counter}`
/// * Additional values: `_CONSTRUCT_{term_counter}`
/// * Aggregates: `_AGGREGATE_{term_counter}`
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ChaseRule {
    /// Positive part of the body
    positive_body: Vec<VariableAtom>,
    /// Derived bindings from the positive body
    /// These should appear in order,
    /// i.e. such that the computation of a value
    /// does not depend on values constructed later
    positive_constructors: Vec<Constructor>,
    /// Restriction on the positive part of the body
    positive_constraints: Vec<Constraint>,

    /// Negative part of the body
    negative_body: Vec<VariableAtom>,
    /// For each [VariableAtom] in `negative_body`,
    /// the associated filter statements
    negative_constraints: Vec<Vec<Constraint>>,

    /// Aggregate
    aggregate: Option<ChaseAggregate>,

    /// Constructors from aggregate results
    aggregate_constructors: Vec<Constructor>,
    /// Restraints on values constructed from aggregate results
    aggregate_constraints: Vec<Constraint>,

    /// Head atoms of the rule
    head: Vec<PrimitiveAtom>,
    /// Index of the head atom which contains the aggregate
    aggregate_head_index: Option<usize>,
}

#[allow(dead_code)]
impl ChaseRule {
    /// Construct a new [ChaseRule].
    pub fn positive_rule(
        head: Vec<PrimitiveAtom>,
        positive_body: Vec<VariableAtom>,
        positive_constraints: Vec<Constraint>,
    ) -> Self {
        Self {
            positive_body,
            positive_constructors: vec![],
            positive_constraints,
            negative_body: vec![],
            negative_constraints: vec![],
            aggregate: None,
            aggregate_constructors: vec![],
            aggregate_constraints: vec![],
            head,
            aggregate_head_index: None,
        }
    }

    /// Return the head atoms of the rule - immutable.
    #[must_use]
    pub fn head(&self) -> &Vec<PrimitiveAtom> {
        &self.head
    }

    /// Return the head atoms of the rule - mutable.
    #[must_use]
    pub fn head_mut(&mut self) -> &mut Vec<PrimitiveAtom> {
        &mut self.head
    }

    /// Return the positive constructors of the rule.
    pub fn positive_constructors(&self) -> &Vec<Constructor> {
        &self.positive_constructors
    }

    /// Return the aggregate of the rule.
    pub fn aggregate(&self) -> &Option<ChaseAggregate> {
        &self.aggregate
    }

    /// Return the index of the aggregate head atom.
    pub fn aggregate_head_index(&self) -> Option<usize> {
        self.aggregate_head_index
    }

    /// Return the all the atoms of the rules.
    /// This does not distinguish between positive and negative atoms.
    pub fn all_body(&self) -> impl Iterator<Item = &VariableAtom> {
        self.positive_body.iter().chain(self.negative_body.iter())
    }

    /// Return the positive body atoms of the rule - immutable.
    #[must_use]
    pub fn positive_body(&self) -> &Vec<VariableAtom> {
        &self.positive_body
    }

    /// Return the positive body atoms of the rule - mutable.
    #[must_use]
    pub fn positive_body_mut(&mut self) -> &mut Vec<VariableAtom> {
        &mut self.positive_body
    }

    /// Return all the constraints of the rule.
    pub fn all_constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.positive_constraints
            .iter()
            .chain(self.negative_constraints.iter().flatten())
            .chain(self.aggregate_constraints.iter())
    }

    /// Return the positive constraints of the rule - immutable.
    #[must_use]
    pub fn positive_constraints(&self) -> &Vec<Constraint> {
        &self.positive_constraints
    }

    /// Return the positive constraints of the rule - mutable.
    #[must_use]
    pub fn positive_constraints_mut(&mut self) -> &mut Vec<Constraint> {
        &mut self.positive_constraints
    }

    /// Return the negative body atons of the rule - immutable.
    #[must_use]
    pub fn negative_body(&self) -> &Vec<VariableAtom> {
        &self.negative_body
    }

    /// Return the negative body atoms of the rule - mutable.
    #[must_use]
    pub fn negative_body_mut(&mut self) -> &mut Vec<VariableAtom> {
        &mut self.negative_body
    }

    /// Return the negative constraints of the rule - immutable.
    #[must_use]
    pub fn negative_constraints(&self) -> &Vec<Vec<Constraint>> {
        &self.negative_constraints
    }

    /// Return the negative constraints of the rule - mutable.
    #[must_use]
    pub fn negative_constraints_mut(&mut self) -> &mut Vec<Vec<Constraint>> {
        &mut self.negative_constraints
    }

    /// Return the aggregate constraints of the rule.
    #[must_use]
    pub fn aggregate_constraints(&self) -> &Vec<Constraint> {
        &self.aggregate_constraints
    }

    /// Return the aggregate constraints of the rule.
    #[must_use]
    pub fn aggregate_constructors(&self) -> &Vec<Constructor> {
        &self.aggregate_constructors
    }

    /// Return all [Variable]s used in this rule.
    pub fn all_variables(&self) -> Vec<Variable> {
        let variables_body = self.all_body().flat_map(|atom| atom.get_variables());
        let variables_head = self.head.iter().flat_map(|atom| atom.get_variables());
        let variables_constructors = self
            .positive_constructors
            .iter()
            .chain(self.aggregate_constructors.iter())
            .map(|constructor| constructor.variable().clone());
        let variables_aggregates = self
            .aggregate
            .iter()
            .map(|aggregate| aggregate.output_variable.clone());

        variables_body
            .chain(variables_head)
            .chain(variables_constructors)
            .chain(variables_aggregates)
            .collect()
    }

    /// Returns the [AnyDataValue]s used as constants in this rule.
    pub fn all_datavalues(&self) -> impl Iterator<Item = &AnyDataValue> {
        let datavalues_head = self.head.iter().flat_map(|atom| atom.datavalues());
        let datavalues_constructors = self
            .positive_constructors
            .iter()
            .chain(self.aggregate_constructors.iter())
            .flat_map(|constructor| constructor.datavalues());
        let datavalues_constraints = self
            .positive_constraints
            .iter()
            .chain(self.negative_constraints.iter().flatten())
            .chain(self.aggregate_constraints.iter())
            .flat_map(|constraint| constraint.datavalues());

        datavalues_head
            .chain(datavalues_constructors)
            .chain(datavalues_constraints)
    }
}

/// Helper structure defining several categories of constraints
#[derive(Debug)]
struct ConstraintCategories {
    positive_constructors: Vec<Constructor>,
    positive_constraints: Vec<Constraint>,
    negative_constraints: Vec<Vec<Constraint>>,
    aggregate_constructors: Vec<Constructor>,
    aggregate_constraints: Vec<Constraint>,
}

impl ConstraintCategories {
    pub fn new(num_negative_body: usize) -> Self {
        Self {
            positive_constructors: Vec::<Constructor>::new(),
            positive_constraints: Vec::<Constraint>::new(),
            negative_constraints: vec![Vec::<Constraint>::new(); num_negative_body],
            aggregate_constructors: Vec::<Constructor>::new(),
            aggregate_constraints: Vec::<Constraint>::new(),
        }
    }
}

impl ChaseRule {
    /// Increments `next_variable_id`, but returns it's old value with a prefix.
    fn generate_incrementing_variable_name(prefix: &str, next_variable_id: &mut usize) -> String {
        let result = format!("{}{}", prefix, next_variable_id);
        *next_variable_id += 1;
        result
    }

    // Remove constraints of the form ?X = ?Y from the rule
    // and apply the corresponding substitution
    fn apply_equality(rule: &mut Rule) {
        let mut assignment = HashMap::<Variable, Term>::new();

        rule.constraints_mut().retain(|constraint| {
            if let Constraint::Equals(
                Term::Primitive(PrimitiveTerm::Variable(left)),
                Term::Primitive(PrimitiveTerm::Variable(right)),
            ) = constraint
            {
                if let Some(assigned) = assignment.get(left) {
                    assignment.insert(right.clone(), assigned.clone());
                } else if let Some(assigned) = assignment.get(right) {
                    assignment.insert(left.clone(), assigned.clone());
                } else {
                    assignment.insert(
                        left.clone(),
                        Term::Primitive(PrimitiveTerm::Variable(right.clone())),
                    );
                }

                return false;
            }

            true
        });

        rule.apply_assignment(&assignment);
    }

    /// Modify the rule in such a way
    /// that it only contains primitive terms in the head
    /// and variables in the body.
    ///
    /// This transformation may introduce new [Constraint]s.
    fn flatten_atoms(
        rule: &mut Rule,
        aggregate: &mut Option<ChaseAggregate>,
        aggregate_head_index: &mut Option<usize>,
    ) -> ConstraintCategories {
        let num_negative_body = rule.num_negative_body();
        let mut new_constraints = ConstraintCategories::new(num_negative_body);

        let mut rule_next_variable_id: usize = 0;

        // Head atoms may only contain primitive terms
        // Aggregates need to be separated
        for (atom_index, atom) in rule.head_mut().iter_mut().enumerate() {
            struct AggregateInformation {
                term_index: usize,
                aggregate: Aggregate,
                output_variable: Variable,
                surrounding_term: Option<Term>,
            }
            let mut aggregate_information: Option<AggregateInformation> = None;

            for (term_index, term) in atom.terms_mut().iter_mut().enumerate() {
                // Replace aggregate terms or aggregates inside of arithmetic expressions with placeholder variables
                term.update_subterms_recursively(&mut |subterm| match subterm {
                    Term::Aggregation(aggregate) => {
                        let output_variable =
                            Variable::Universal(Self::generate_incrementing_variable_name(
                                AGGREGATE_VARIABLE_PREFIX,
                                &mut rule_next_variable_id,
                            ));

                        for aggregate_subterm in &mut aggregate.terms {
                            if !aggregate_subterm.is_variable() {
                                let new_variable =
                                    Variable::Universal(Self::generate_incrementing_variable_name(
                                        CONSTRUCT_VARIABLE_PREFIX,
                                        &mut rule_next_variable_id,
                                    ));
                                let new_term =
                                    Term::Primitive(PrimitiveTerm::Variable(new_variable.clone()));

                                new_constraints.positive_constructors.push(Constructor::new(
                                    new_variable,
                                    aggregate_subterm.clone(),
                                ));
                                *aggregate_subterm = new_term;
                            }
                        }

                        aggregate_information = Some(AggregateInformation {
                            term_index,
                            aggregate: aggregate.clone(),
                            output_variable: output_variable.clone(),
                            surrounding_term: None,
                        });

                        *subterm = Term::Primitive(PrimitiveTerm::Variable(output_variable));

                        false
                    }
                    _ => true,
                });

                debug_assert!(
                    !matches!(term, Term::Aggregation(_)),
                    "Aggregate terms should have been replaced with placeholder variables"
                );

                if !term.is_primitive() {
                    let new_variable =
                        Variable::Universal(Self::generate_incrementing_variable_name(
                            CONSTRUCT_VARIABLE_PREFIX,
                            &mut rule_next_variable_id,
                        ));
                    let new_term = Term::Primitive(PrimitiveTerm::Variable(new_variable.clone()));

                    let is_aggregate =
                        if let Some(aggregate_information) = &mut aggregate_information {
                            aggregate_information.surrounding_term = Some(term.clone());

                            aggregate_information.term_index == term_index
                        } else {
                            false
                        };

                    if is_aggregate {
                        new_constraints
                            .aggregate_constructors
                            .push(Constructor::new(new_variable, term.clone()));
                    } else {
                        new_constraints
                            .positive_constructors
                            .push(Constructor::new(new_variable, term.clone()));
                    }

                    *term = new_term;
                }
            }

            if let Some(information) = aggregate_information {
                let mut group_by_variables = HashSet::<Variable>::new();
                for (term_index, term) in atom.terms().iter().enumerate() {
                    if term_index == information.term_index {
                        continue;
                    }

                    if let Term::Primitive(PrimitiveTerm::Variable(variable)) = term {
                        group_by_variables.insert(variable.clone());
                    }
                }

                if let Some(surrounding_term) = information.surrounding_term {
                    group_by_variables.extend(surrounding_term.variables().cloned());
                    group_by_variables.remove(&information.output_variable);
                }

                *aggregate = Some(ChaseAggregate::from_aggregate(
                    information.aggregate,
                    information.output_variable,
                    group_by_variables,
                ));
                *aggregate_head_index = Some(atom_index);
            }
        }

        // Body literals must only contain variables
        // and may not repeat variables within one atom
        let mut negative_index = 0;
        for literal in rule.body_mut() {
            let is_positive = literal.is_positive();
            if !is_positive {
                negative_index += 1;
            }

            let atom = literal.atom_mut();
            let mut current_variables = HashSet::<Variable>::new();

            for term in atom.terms_mut() {
                if let Term::Primitive(PrimitiveTerm::Variable(variable)) = term.clone() {
                    if !current_variables.contains(&variable) {
                        current_variables.insert(variable);

                        continue;
                    }
                }

                let new_variable = Variable::Universal(Self::generate_incrementing_variable_name(
                    EQUALITY_VARIABLE_PREFIX,
                    &mut rule_next_variable_id,
                ));
                let new_term = Term::Primitive(PrimitiveTerm::Variable(new_variable.clone()));

                if is_positive {
                    new_constraints
                        .positive_constraints
                        .push(Constraint::Equals(new_term.clone(), term.clone()));
                } else {
                    new_constraints.negative_constraints[negative_index - 1]
                        .push(Constraint::Equals(new_term.clone(), term.clone()))
                }

                *term = new_term;
            }
        }

        new_constraints
    }

    fn compute_derived_variables(
        rule: &Rule,
        aggregate: &Option<ChaseAggregate>,
        constraints: &mut ConstraintCategories,
        assigned_constraints: &mut HashSet<usize>,
    ) -> HashSet<Variable> {
        let mut derived_variables = rule.safe_variables();
        let mut aggregate_variables = HashSet::<Variable>::new();
        if let Some(aggregate) = aggregate {
            aggregate_variables.insert(aggregate.output_variable.clone());
        }

        let mut update = true;
        while update {
            let num_assigned_constraints = assigned_constraints.len();

            for (constraint_index, constraint) in rule.constraints().iter().enumerate() {
                if assigned_constraints.contains(&constraint_index) {
                    continue;
                }

                if let Some((variable, term)) = constraint.has_form_assignment() {
                    if derived_variables.contains(variable)
                        || aggregate_variables.contains(variable)
                    {
                        continue;
                    }

                    if !derived_variables.contains(variable)
                        && term
                            .variables()
                            .all(|variable| derived_variables.contains(variable))
                    {
                        derived_variables.insert(variable.clone());
                        constraints
                            .positive_constructors
                            .push(Constructor::new(variable.clone(), term.clone()));
                        assigned_constraints.insert(constraint_index);
                        continue;
                    }

                    if !aggregate_variables.contains(variable)
                        && term.variables().all(|variable| {
                            derived_variables.contains(variable)
                                || aggregate_variables.contains(variable)
                        })
                    {
                        aggregate_variables.insert(variable.clone());
                        constraints
                            .aggregate_constructors
                            .push(Constructor::new(variable.clone(), term.clone()));
                        assigned_constraints.insert(constraint_index);
                        continue;
                    }
                }
            }

            update = num_assigned_constraints != assigned_constraints.len();
        }

        derived_variables
    }

    /// Seperate different [Constraint]s of the given [Rule] into several categories.
    fn seperate_constraints(
        rule: &Rule,
        aggregate: &Option<ChaseAggregate>,
        negative_body: &[VariableAtom],
        constraints: &mut ConstraintCategories,
    ) {
        let mut assigned_constraints = HashSet::<usize>::new();
        let derived_variables = Self::compute_derived_variables(
            rule,
            aggregate,
            constraints,
            &mut assigned_constraints,
        );

        let mut negative_variables = HashMap::<Variable, usize>::new();
        for (body_index, negative_atom) in negative_body.iter().enumerate() {
            for variable in negative_atom.terms() {
                if !derived_variables.contains(variable) {
                    negative_variables.insert(variable.clone(), body_index);
                }
            }
        }

        for (constraint_index, constraint) in rule.constraints().iter().enumerate() {
            if assigned_constraints.contains(&constraint_index) {
                continue;
            }

            // Constraint on derived variables
            if constraint
                .variables()
                .all(|variable| derived_variables.contains(variable))
            {
                constraints.positive_constraints.push(constraint.clone());
                assigned_constraints.insert(constraint_index);
                continue;
            }

            // Constraint on negative variables
            for variable in constraint.variables() {
                if let Some(negative_index) = negative_variables.get(variable) {
                    constraints.negative_constraints[*negative_index].push(constraint.clone());
                    assigned_constraints.insert(constraint_index);
                    continue;
                }
            }

            // Constraints on aggregates are currently not expressible
        }

        debug_assert!(assigned_constraints.len() == rule.constraints().len());
    }
}

impl TryFrom<Rule> for ChaseRule {
    type Error = Error;

    fn try_from(mut rule: Rule) -> Result<ChaseRule, Error> {
        // Preprocess rule in order to make the translation simpler
        let mut aggregate: Option<ChaseAggregate> = None;
        let mut aggregate_head_index: Option<usize> = None;

        Self::apply_equality(&mut rule);
        let mut constraints =
            Self::flatten_atoms(&mut rule, &mut aggregate, &mut aggregate_head_index);

        // Build chase rule elements from flattend atoms
        let head = rule
            .head()
            .iter()
            .map(PrimitiveAtom::from_flat_atom)
            .collect::<Vec<PrimitiveAtom>>();

        let mut positive_body = Vec::new();
        let mut negative_body = Vec::new();
        for literal in rule.body() {
            match literal {
                Literal::Positive(atom) => positive_body.push(VariableAtom::from_flat_atom(atom)),
                Literal::Negative(atom) => negative_body.push(VariableAtom::from_flat_atom(atom)),
            }
        }

        // Seperate constraints into different categories
        Self::seperate_constraints(&rule, &aggregate, &negative_body, &mut constraints);

        let ConstraintCategories {
            positive_constructors,
            positive_constraints,
            negative_constraints,
            aggregate_constructors,
            aggregate_constraints,
        } = constraints;

        Ok(Self {
            positive_body,
            positive_constructors,
            positive_constraints,
            negative_body,
            negative_constraints,
            aggregate,
            aggregate_constructors,
            aggregate_constraints,
            head,
            aggregate_head_index,
        })
    }
}
