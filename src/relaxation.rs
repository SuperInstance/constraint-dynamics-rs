//! Constraint relaxation for over-constrained systems.
//!
//! When a CSP has no solution, we can relax constraints (by reducing strength or removing them)
//! to find the best partial solution.

use crate::constraint::{Constraint, Variable};
use crate::solver::{Solver, Solution};

/// Result of a relaxed solve attempt.
#[derive(Debug, Clone)]
pub struct RelaxedSolution {
    /// The assignments found.
    pub assignments: Vec<(usize, i64)>,
    /// Total weight of satisfied constraints.
    pub satisfied_weight: f64,
    /// Total weight of all constraints.
    pub total_weight: f64,
    /// Indices of violated constraints.
    pub violated: Vec<usize>,
}

impl RelaxedSolution {
    /// Satisfaction ratio: what fraction of constraint weight is satisfied.
    pub fn satisfaction_ratio(&self) -> f64 {
        if self.total_weight == 0.0 {
            1.0
        } else {
            self.satisfied_weight / self.total_weight
        }
    }
}

/// Relaxation strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum RelaxationStrategy {
    /// Remove the weakest constraint first.
    WeakestFirst,
    /// Remove constraints in order (last added first).
    ReverseOrder,
    /// Remove a specific constraint by index.
    RemoveIndex(usize),
    /// Reduce strength of all constraints by a factor.
    ReduceStrength(f64),
}

/// Attempt to solve an over-constrained system by relaxing constraints.
pub fn relaxed_solve(
    variables: &[Variable],
    constraints: &[Constraint],
    strategy: &RelaxationStrategy,
) -> Option<RelaxedSolution> {
    let solver = Solver::new();

    match strategy {
        RelaxationStrategy::WeakestFirst => {
            let mut sorted: Vec<(usize, &Constraint)> = constraints.iter().enumerate().collect();
            sorted.sort_by(|a, b| a.1.strength.partial_cmp(&b.1.strength).unwrap());

            // Try removing constraints from weakest to strongest
            for remove_count in 0..=constraints.len() {
                let to_remove: std::collections::HashSet<usize> = sorted
                    .iter()
                    .take(remove_count)
                    .map(|(i, _)| *i)
                    .collect();

                let remaining: Vec<Constraint> = constraints
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !to_remove.contains(i))
                    .map(|(_, c)| c.clone())
                    .collect();

                if let Some(sol) = solver.solve(variables, &remaining) {
                    return Some(build_relaxed_solution(&sol, constraints, &to_remove));
                }
            }
            None
        }
        RelaxationStrategy::ReverseOrder => {
            for remove_count in 0..=constraints.len() {
                let to_remove: std::collections::HashSet<usize> =
                    (constraints.len() - remove_count..constraints.len()).collect();

                let remaining: Vec<Constraint> = constraints
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !to_remove.contains(i))
                    .map(|(_, c)| c.clone())
                    .collect();

                if let Some(sol) = solver.solve(variables, &remaining) {
                    return Some(build_relaxed_solution(&sol, constraints, &to_remove));
                }
            }
            None
        }
        RelaxationStrategy::RemoveIndex(idx) => {
            let remaining: Vec<Constraint> = constraints
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != *idx)
                .map(|(_, c)| c.clone())
                .collect();

            solver.solve(variables, &remaining).map(|sol| {
                build_relaxed_solution(&sol, constraints, &[*idx].into_iter().collect())
            })
        }
        RelaxationStrategy::ReduceStrength(factor) => {
            let relaxed: Vec<Constraint> = constraints
                .iter()
                .map(|c| {
                    let mut rc = c.clone();
                    rc.strength *= factor;
                    rc
                })
                .collect();

            solver
                .solve(variables, &relaxed)
                .map(|sol| RelaxedSolution {
                    assignments: sol.assignments,
                    satisfied_weight: relaxed.iter().map(|c| c.strength).sum(),
                    total_weight: relaxed.iter().map(|c| c.strength).sum(),
                    violated: vec![],
                })
        }
    }
}

fn build_relaxed_solution(
    sol: &Solution,
    constraints: &[Constraint],
    removed: &std::collections::HashSet<usize>,
) -> RelaxedSolution {
    let mut satisfied_weight = 0.0;
    let mut total_weight = 0.0;
    let mut violated = Vec::new();

    for (i, c) in constraints.iter().enumerate() {
        total_weight += c.strength;
        if removed.contains(&i) {
            violated.push(i);
        } else {
            let vals: Vec<i64> = c
                .variables
                .iter()
                .map(|vid| {
                    sol.assignments
                        .iter()
                        .find(|(id, _)| *id == *vid)
                        .unwrap()
                        .1
                })
                .collect();
            if c.is_satisfied(&vals) {
                satisfied_weight += c.strength;
            } else {
                violated.push(i);
            }
        }
    }

    RelaxedSolution {
        assignments: sol.assignments.clone(),
        satisfied_weight,
        total_weight,
        violated,
    }
}

/// Find the minimal set of constraints to relax to achieve a solution.
pub fn minimal_relaxation(
    variables: &[Variable],
    constraints: &[Constraint],
) -> Option<RelaxedSolution> {
    relaxed_solve(variables, constraints, &RelaxationStrategy::WeakestFirst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{Constraint, Variable};

    #[test]
    fn test_relaxation_over_constrained() {
        let vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1, 2]),
            Variable::new(2, vec![1, 2]),
        ];
        let constraints = vec![
            Constraint::equality(0, 0, 1, vec![], 1.0),  // forces 1=1
            Constraint::inequality(1, 1, 2, vec![], 1.0), // forces 1≠2 — ok
            Constraint::equality(2, 0, 2, vec![], 0.5),   // forces 1=1 or 1=2
        ];
        // var 0 = 1, var 1 = 1 (from eq), var 2 must be ≠1 and =1 — contradiction
        let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::WeakestFirst);
        assert!(result.is_some());
        let sol = result.unwrap();
        assert!(sol.satisfaction_ratio() > 0.0);
    }

    #[test]
    fn test_relaxation_remove_index() {
        let vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1]),
        ];
        let constraints = vec![
            Constraint::inequality(0, 0, 1, vec![], 1.0),
        ];
        let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::RemoveIndex(0));
        assert!(result.is_some());
        let sol = result.unwrap();
        assert!(sol.violated.contains(&0));
    }

    #[test]
    fn test_relaxation_no_need() {
        let vars = vec![
            Variable::new(0, vec![1, 2]),
            Variable::new(1, vec![1, 2]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        let result = relaxed_solve(&vars, &[c], &RelaxationStrategy::WeakestFirst);
        assert!(result.is_some());
        assert_eq!(result.unwrap().satisfaction_ratio(), 1.0);
    }

    #[test]
    fn test_satisfaction_ratio_zero_weight() {
        let sol = RelaxedSolution {
            assignments: vec![(0, 1)],
            satisfied_weight: 0.0,
            total_weight: 0.0,
            violated: vec![],
        };
        assert_eq!(sol.satisfaction_ratio(), 1.0);
    }

    #[test]
    fn test_minimal_relaxation() {
        let vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![2]),
        ];
        let constraints = vec![
            Constraint::equality(0, 0, 1, vec![], 1.0),
        ];
        let result = minimal_relaxation(&vars, &constraints);
        assert!(result.is_some());
    }

    #[test]
    fn test_reduce_strength() {
        let vars = vec![
            Variable::new(0, vec![1, 2]),
            Variable::new(1, vec![1, 2]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        let result = relaxed_solve(&vars, &[c], &RelaxationStrategy::ReduceStrength(0.5));
        assert!(result.is_some());
        assert_eq!(result.unwrap().satisfaction_ratio(), 1.0);
    }
}
