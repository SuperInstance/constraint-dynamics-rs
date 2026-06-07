//! Core constraint types and structures.

use std::fmt;
use std::rc::Rc;

/// The type of constraint.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintType {
    /// Unary constraint on a single variable.
    Unary,
    /// Binary constraint between two variables.
    Binary,
    /// N-ary constraint over multiple variables.
    Nary(usize),
}

/// A constraint over variables in a domain.
pub struct Constraint {
    /// Unique identifier for this constraint.
    pub id: usize,
    /// Type of constraint.
    pub constraint_type: ConstraintType,
    /// Strength of the constraint (0.0 = weak, 1.0 = mandatory).
    pub strength: f64,
    /// Variables involved in this constraint.
    pub variables: Vec<usize>,
    /// Domain values available for each variable (shared reference by index).
    pub domain: Vec<Vec<i64>>,
    /// The predicate function: given an assignment of variables, returns true if satisfied.
    #[allow(clippy::type_complexity)]
    predicate: Rc<dyn Fn(&[i64]) -> bool>,
}

impl Clone for Constraint {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            constraint_type: self.constraint_type.clone(),
            strength: self.strength,
            variables: self.variables.clone(),
            domain: self.domain.clone(),
            predicate: self.predicate.clone(),
        }
    }
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Constraint")
            .field("id", &self.id)
            .field("constraint_type", &self.constraint_type)
            .field("strength", &self.strength)
            .field("variables", &self.variables)
            .finish()
    }
}

impl Constraint {
    /// Create a new constraint.
    pub fn new(
        id: usize,
        variables: Vec<usize>,
        domain: Vec<Vec<i64>>,
        strength: f64,
        predicate: impl Fn(&[i64]) -> bool + 'static,
    ) -> Self {
        let constraint_type = match variables.len() {
            1 => ConstraintType::Unary,
            2 => ConstraintType::Binary,
            n => ConstraintType::Nary(n),
        };
        Self {
            id,
            constraint_type,
            strength,
            variables,
            domain,
            predicate: Rc::new(predicate),
        }
    }

    /// Check if an assignment satisfies this constraint.
    pub fn is_satisfied(&self, assignment: &[i64]) -> bool {
        (self.predicate)(assignment)
    }

    /// Get the arity of this constraint.
    pub fn arity(&self) -> usize {
        self.variables.len()
    }

    /// Check if the constraint is mandatory (strength == 1.0).
    pub fn is_mandatory(&self) -> bool {
        self.strength >= 1.0
    }

    /// Create an equality constraint between two variables.
    pub fn equality(id: usize, var_a: usize, var_b: usize, domain: Vec<Vec<i64>>, strength: f64) -> Self {
        Self::new(id, vec![var_a, var_b], domain, strength, |vals| vals[0] == vals[1])
    }

    /// Create an inequality constraint between two variables.
    pub fn inequality(id: usize, var_a: usize, var_b: usize, domain: Vec<Vec<i64>>, strength: f64) -> Self {
        Self::new(id, vec![var_a, var_b], domain, strength, |vals| vals[0] != vals[1])
    }

    /// Create a less-than constraint.
    pub fn less_than(id: usize, var_a: usize, var_b: usize, domain: Vec<Vec<i64>>, strength: f64) -> Self {
        Self::new(id, vec![var_a, var_b], domain, strength, |vals| vals[0] < vals[1])
    }

    /// Create a range constraint (unary): variable must be within [lo, hi].
    pub fn range(
        id: usize,
        var: usize,
        domain: Vec<Vec<i64>>,
        strength: f64,
        lo: i64,
        hi: i64,
    ) -> Self {
        Self::new(id, vec![var], domain, strength, move |vals| vals[0] >= lo && vals[0] <= hi)
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint(id={}, vars={:?}, strength={:.2}, type={:?})",
            self.id, self.variables, self.strength, self.constraint_type
        )
    }
}

/// A variable in the constraint system.
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable index.
    pub id: usize,
    /// Current domain of possible values.
    pub domain: Vec<i64>,
    /// Current assignment (if any).
    pub value: Option<i64>,
}

impl Variable {
    /// Create a new variable with the given domain.
    pub fn new(id: usize, domain: Vec<i64>) -> Self {
        Self { id, domain, value: None }
    }

    /// Assign a value to this variable.
    pub fn assign(&mut self, value: i64) {
        self.value = Some(value);
    }

    /// Unassign this variable.
    pub fn unassign(&mut self) {
        self.value = None;
    }

    /// Check if assigned.
    pub fn is_assigned(&self) -> bool {
        self.value.is_some()
    }

    /// Remove a value from the domain.
    pub fn remove_from_domain(&mut self, val: i64) -> bool {
        if let Some(pos) = self.domain.iter().position(|&v| v == val) {
            self.domain.remove(pos);
            true
        } else {
            false
        }
    }

    /// Domain size.
    pub fn domain_size(&self) -> usize {
        self.domain.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_new_binary() {
        let c = Constraint::equality(0, 0, 1, vec![vec![1, 2, 3], vec![1, 2, 3]], 1.0);
        assert_eq!(c.id, 0);
        assert_eq!(c.constraint_type, ConstraintType::Binary);
        assert_eq!(c.strength, 1.0);
        assert_eq!(c.arity(), 2);
        assert!(c.is_mandatory());
    }

    #[test]
    fn test_constraint_unary() {
        let c = Constraint::range(1, 0, vec![vec![1, 5, 10]], 0.8, 2, 8);
        assert_eq!(c.constraint_type, ConstraintType::Unary);
        assert_eq!(c.arity(), 1);
        assert!(!c.is_mandatory());
    }

    #[test]
    fn test_equality_satisfied() {
        let c = Constraint::equality(0, 0, 1, vec![vec![1, 2], vec![1, 2]], 1.0);
        assert!(c.is_satisfied(&[2, 2]));
        assert!(!c.is_satisfied(&[1, 2]));
    }

    #[test]
    fn test_inequality_satisfied() {
        let c = Constraint::inequality(0, 0, 1, vec![vec![1, 2], vec![1, 2]], 1.0);
        assert!(c.is_satisfied(&[1, 2]));
        assert!(!c.is_satisfied(&[1, 1]));
    }

    #[test]
    fn test_less_than() {
        let c = Constraint::less_than(0, 0, 1, vec![vec![1, 2], vec![1, 2]], 1.0);
        assert!(c.is_satisfied(&[1, 2]));
        assert!(!c.is_satisfied(&[2, 1]));
    }

    #[test]
    fn test_range_constraint() {
        let c = Constraint::range(0, 0, vec![vec![1, 5, 10]], 1.0, 2, 8);
        assert!(c.is_satisfied(&[5]));
        assert!(!c.is_satisfied(&[1]));
        assert!(!c.is_satisfied(&[10]));
    }

    #[test]
    fn test_variable_assign_unassign() {
        let mut v = Variable::new(0, vec![1, 2, 3]);
        assert!(!v.is_assigned());
        v.assign(2);
        assert!(v.is_assigned());
        assert_eq!(v.value, Some(2));
        v.unassign();
        assert!(!v.is_assigned());
    }

    #[test]
    fn test_variable_remove_from_domain() {
        let mut v = Variable::new(0, vec![1, 2, 3]);
        assert!(v.remove_from_domain(2));
        assert_eq!(v.domain, vec![1, 3]);
        assert!(!v.remove_from_domain(5));
        assert_eq!(v.domain_size(), 2);
    }

    #[test]
    fn test_display() {
        let c = Constraint::equality(42, 0, 1, vec![], 0.5);
        let s = format!("{}", c);
        assert!(s.contains("42"));
        assert!(s.contains("0.50"));
    }
}
