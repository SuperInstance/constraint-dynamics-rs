//! Backtracking solver with constraint propagation.

use crate::constraint::{Constraint, Variable};
use crate::dynamics::forward_check;

/// Result of solving a constraint satisfaction problem.
#[derive(Debug, Clone)]
pub struct Solution {
    /// Variable assignments: (variable_id, value).
    pub assignments: Vec<(usize, i64)>,
    /// Number of backtracks performed.
    pub backtracks: usize,
}

/// A backtracking CSP solver with optional constraint propagation.
pub struct Solver {
    /// Whether to use forward checking during search.
    pub use_forward_check: bool,
    /// Maximum backtracks before giving up (0 = unlimited).
    pub max_backtracks: usize,
}

impl Solver {
    /// Create a new solver.
    pub fn new() -> Self {
        Self {
            use_forward_check: true,
            max_backtracks: 0,
        }
    }

    /// Solve the CSP.
    pub fn solve(&self, variables: &[Variable], constraints: &[Constraint]) -> Option<Solution> {
        let mut vars: Vec<Variable> = variables.to_vec();
        let mut backtracks = 0;
        let result = self.backtrack(&mut vars, constraints, &mut backtracks)?;
        Some(Solution {
            assignments: result,
            backtracks,
        })
    }

    fn backtrack(
        &self,
        vars: &mut [Variable],
        constraints: &[Constraint],
        backtracks: &mut usize,
    ) -> Option<Vec<(usize, i64)>> {
        // Check if all variables are assigned
        if vars.iter().all(|v| v.is_assigned()) {
            // Verify all constraints
            if self.all_constraints_satisfied(vars, constraints) {
                return Some(
                    vars.iter()
                        .map(|v| (v.id, v.value.unwrap()))
                        .collect(),
                );
            }
            return None;
        }

        // Select unassigned variable (MRV heuristic: smallest domain)
        let var_idx = self.select_variable(vars);
        let domain: Vec<i64> = vars[var_idx].domain.clone();

        for val in domain {
            if self.max_backtracks > 0 && *backtracks >= self.max_backtracks {
                return None;
            }

            vars[var_idx].assign(val);

            if self.is_consistent(var_idx, vars, constraints) {
                if self.use_forward_check {
                    // Save domains
                    let saved_domains: Vec<Vec<i64>> =
                        vars.iter().map(|v| v.domain.clone()).collect();

                    if forward_check(vars, constraints, var_idx) {
                        if let Some(result) = self.backtrack(vars, constraints, backtracks) {
                            return Some(result);
                        }
                    }

                    // Restore domains
                    for (i, v) in vars.iter_mut().enumerate() {
                        v.domain = saved_domains[i].clone();
                        if i != var_idx {
                            v.value = None;
                        }
                    }
                } else {
                    if let Some(result) = self.backtrack(vars, constraints, backtracks) {
                        return Some(result);
                    }
                }
            }

            vars[var_idx].unassign();
            *backtracks += 1;
        }

        None
    }

    fn select_variable(&self, vars: &[Variable]) -> usize {
        vars.iter()
            .enumerate()
            .filter(|(_, v)| !v.is_assigned())
            .min_by_key(|(_, v)| v.domain_size())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn is_consistent(&self, var_idx: usize, vars: &[Variable], constraints: &[Constraint]) -> bool {
        let var_id = vars[var_idx].id;
        for c in constraints {
            if !c.variables.contains(&var_id) {
                continue;
            }
            // Only check if all variables in the constraint are assigned
            let assigned: Vec<Option<i64>> = c
                .variables
                .iter()
                .map(|&vid| vars.iter().find(|v| v.id == vid).and_then(|v| v.value))
                .collect();

            if assigned.iter().all(|v| v.is_some()) {
                let vals: Vec<i64> = assigned.iter().map(|v| v.unwrap()).collect();
                if !c.is_satisfied(&vals) {
                    return false;
                }
            } else {
                // Partial check: verify assigned variables don't already conflict
                // For binary constraints, check the assigned side
                if c.arity() == 2 {
                    let my_val = vars[var_idx].value.unwrap();
                    let other_var_id = if c.variables[0] == var_id {
                        c.variables[1]
                    } else {
                        c.variables[0]
                    };
                    if let Some(other) = vars.iter().find(|v| v.id == other_var_id) {
                        if let Some(other_val) = other.value {
                            let vals = if c.variables[0] == var_id {
                                vec![my_val, other_val]
                            } else {
                                vec![other_val, my_val]
                            };
                            if !c.is_satisfied(&vals) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn all_constraints_satisfied(&self, vars: &[Variable], constraints: &[Constraint]) -> bool {
        for c in constraints {
            let vals: Vec<i64> = c
                .variables
                .iter()
                .map(|&vid| vars.iter().find(|v| v.id == vid).unwrap().value.unwrap())
                .collect();
            if !c.is_satisfied(&vals) {
                return false;
            }
        }
        true
    }

    /// Count all solutions (up to a limit).
    pub fn count_solutions(
        &self,
        variables: &[Variable],
        constraints: &[Constraint],
        limit: usize,
    ) -> usize {
        let mut vars = variables.to_vec();
        let mut count = 0;
        self.count_backtrack(&mut vars, constraints, &mut count, limit);
        count
    }

    fn count_backtrack(
        &self,
        vars: &mut [Variable],
        constraints: &[Constraint],
        count: &mut usize,
        limit: usize,
    ) {
        if *count >= limit {
            return;
        }
        if vars.iter().all(|v| v.is_assigned()) {
            if self.all_constraints_satisfied(vars, constraints) {
                *count += 1;
            }
            return;
        }

        let var_idx = self.select_variable(vars);
        let domain = vars[var_idx].domain.clone();

        for val in domain {
            vars[var_idx].assign(val);
            if self.is_consistent(var_idx, vars, constraints) {
                self.count_backtrack(vars, constraints, count, limit);
            }
            vars[var_idx].unassign();
        }
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{Constraint, Variable};

    #[test]
    fn test_solve_simple_equality() {
        let vars = vec![
            Variable::new(0, vec![1, 2, 3]),
            Variable::new(1, vec![1, 2, 3]),
        ];
        let c = Constraint::equality(0, 0, 1, vec![], 1.0);
        let solver = Solver::new();
        let sol = solver.solve(&vars, &[c]).unwrap();
        assert_eq!(sol.assignments[0].1, sol.assignments[1].1);
    }

    #[test]
    fn test_solve_graph_coloring() {
        // 3 variables, all different, domains {1, 2, 3}
        let vars = vec![
            Variable::new(0, vec![1, 2, 3]),
            Variable::new(1, vec![1, 2, 3]),
            Variable::new(2, vec![1, 2, 3]),
        ];
        let constraints = vec![
            Constraint::inequality(0, 0, 1, vec![], 1.0),
            Constraint::inequality(1, 1, 2, vec![], 1.0),
            Constraint::inequality(2, 0, 2, vec![], 1.0),
        ];
        let solver = Solver::new();
        let sol = solver.solve(&vars, &constraints).unwrap();
        let v0 = sol.assignments.iter().find(|(id, _)| *id == 0).unwrap().1;
        let v1 = sol.assignments.iter().find(|(id, _)| *id == 1).unwrap().1;
        let v2 = sol.assignments.iter().find(|(id, _)| *id == 2).unwrap().1;
        assert_ne!(v0, v1);
        assert_ne!(v1, v2);
        assert_ne!(v0, v2);
    }

    #[test]
    fn test_solve_no_solution() {
        let vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        let solver = Solver::new();
        assert!(solver.solve(&vars, &[c]).is_none());
    }

    #[test]
    fn test_solve_less_than_chain() {
        let vars = vec![
            Variable::new(0, vec![1, 2, 3]),
            Variable::new(1, vec![1, 2, 3]),
            Variable::new(2, vec![1, 2, 3]),
        ];
        let constraints = vec![
            Constraint::less_than(0, 0, 1, vec![], 1.0),
            Constraint::less_than(1, 1, 2, vec![], 1.0),
        ];
        let solver = Solver::new();
        let sol = solver.solve(&vars, &constraints).unwrap();
        let v0 = sol.assignments.iter().find(|(id, _)| *id == 0).unwrap().1;
        let v1 = sol.assignments.iter().find(|(id, _)| *id == 1).unwrap().1;
        let v2 = sol.assignments.iter().find(|(id, _)| *id == 2).unwrap().1;
        assert!(v0 < v1);
        assert!(v1 < v2);
    }

    #[test]
    fn test_count_solutions() {
        let vars = vec![
            Variable::new(0, vec![1, 2]),
            Variable::new(1, vec![1, 2]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        let solver = Solver::new();
        assert_eq!(solver.count_solutions(&vars, &[c], 100), 2);
    }

    #[test]
    fn test_max_backtracks() {
        let vars = vec![
            Variable::new(0, vec![1, 2, 3, 4, 5]),
            Variable::new(1, vec![1, 2, 3, 4, 5]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        let mut solver = Solver::new();
        solver.max_backtracks = 2;
        // Should still find a solution with limited backtracks for this simple case
        // or return None if backtracks exceed
        let result = solver.solve(&vars, &[c]);
        // Either finds a solution or gives up — both valid with max_backtracks
        if let Some(sol) = result {
            assert_ne!(sol.assignments[0].1, sol.assignments[1].1);
        }
    }

    #[test]
    fn test_solver_default() {
        let solver = Solver::default();
        assert!(solver.use_forward_check);
        assert_eq!(solver.max_backtracks, 0);
    }
}
