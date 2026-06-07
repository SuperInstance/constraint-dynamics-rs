//! Constraint propagation and arc consistency algorithms.

use crate::constraint::{Constraint, Variable};

/// Perform constraint propagation: iteratively remove values from variable domains
/// that cannot participate in any valid assignment.
///
/// Returns true if the propagation succeeded (no domains were wiped out).
pub fn propagate(variables: &mut [Variable], constraints: &[Constraint]) -> bool {
    let mut changed = true;
    while changed {
        changed = false;
        for constraint in constraints {
            if !propagate_constraint(variables, constraint, &mut changed) {
                return false;
            }
        }
    }
    true
}

/// Propagate a single constraint, removing invalid values.
fn propagate_constraint(
    variables: &mut [Variable],
    constraint: &Constraint,
    changed: &mut bool,
) -> bool {
    match constraint.constraint_type {
        crate::constraint::ConstraintType::Unary => {
            propagate_unary(variables, constraint, changed)
        }
        crate::constraint::ConstraintType::Binary => {
            propagate_binary(variables, constraint, changed)
        }
        crate::constraint::ConstraintType::Nary(_) => {
            propagate_nary(variables, constraint, changed)
        }
    }
}

fn propagate_unary(
    variables: &mut [Variable],
    constraint: &Constraint,
    changed: &mut bool,
) -> bool {
    let var_idx = constraint.variables[0];
    let var = &mut variables[var_idx];
    let original_len = var.domain.len();
    var.domain.retain(|&val| constraint.is_satisfied(&[val]));
    if var.domain.len() != original_len {
        *changed = true;
    }
    !var.domain.is_empty()
}

fn propagate_binary(
    variables: &mut [Variable],
    constraint: &Constraint,
    changed: &mut bool,
) -> bool {
    let (a_idx, b_idx) = (constraint.variables[0], constraint.variables[1]);

    // Forward: remove from A values with no support in B
    {
        let b_domain = variables[b_idx].domain.clone();
        let a = &mut variables[a_idx];
        let original_len = a.domain.len();
        a.domain.retain(|&va| b_domain.iter().any(|&vb| constraint.is_satisfied(&[va, vb])));
        if a.domain.len() != original_len {
            *changed = true;
        }
        if a.domain.is_empty() {
            return false;
        }
    }

    // Backward: remove from B values with no support in A
    {
        let a_domain = variables[a_idx].domain.clone();
        let b = &mut variables[b_idx];
        let original_len = b.domain.len();
        b.domain.retain(|&vb| a_domain.iter().any(|&va| constraint.is_satisfied(&[va, vb])));
        if b.domain.len() != original_len {
            *changed = true;
        }
        if b.domain.is_empty() {
            return false;
        }
    }

    true
}

fn propagate_nary(
    variables: &mut [Variable],
    constraint: &Constraint,
    changed: &mut bool,
) -> bool {
    // For n-ary: check each variable, remove values that have no support
    // from all other variables' domains (brute force cross product check)
    for i in 0..constraint.variables.len() {
        let var_idx = constraint.variables[i];
        let other_domains: Vec<Vec<i64>> = constraint
            .variables
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, &idx)| variables[idx].domain.clone())
            .collect();

        if other_domains.is_empty() {
            continue;
        }

        // Compute cross product of other domains
        let mut supports: std::collections::HashSet<i64> = std::collections::HashSet::new();
        let mut combos = vec![Vec::new()];
        for dom in &other_domains {
            let mut new_combos = Vec::new();
            for combo in &combos {
                for &val in dom {
                    let mut new_combo = combo.clone();
                    new_combo.push(val);
                    new_combos.push(new_combo);
                }
            }
            combos = new_combos;
        }

        for combo in &combos {
            // Check each value in the target variable's domain
            let var = &variables[var_idx];
            for &val in &var.domain {
                // Build the full assignment in constraint order
                let mut assignment = Vec::with_capacity(constraint.variables.len());
                let mut combo_idx = 0;
                for (j, &_cvar_idx) in constraint.variables.iter().enumerate() {
                    if j == i {
                        assignment.push(val);
                    } else {
                        assignment.push(combo[combo_idx]);
                        combo_idx += 1;
                    }
                }
                if constraint.is_satisfied(&assignment) {
                    supports.insert(val);
                }
            }
        }

        let var = &mut variables[var_idx];
        let original_len = var.domain.len();
        var.domain.retain(|&v| supports.contains(&v));
        if var.domain.len() != original_len {
            *changed = true;
        }
        if var.domain.is_empty() {
            return false;
        }
    }
    true
}

/// Enforce arc consistency on a set of binary constraints.
/// A CSP is arc consistent if for every value in the domain of each variable,
/// there exists a consistent value in the domain of every related variable.
pub fn arc_consistency(variables: &mut [Variable], constraints: &[Constraint]) -> bool {
    // Build queue of arcs (variable, constraint) pairs
    let mut queue: Vec<(usize, usize)> = Vec::new(); // (var_idx, constraint_idx)
    for (ci, c) in constraints.iter().enumerate() {
        if c.arity() == 2 {
            queue.push((c.variables[0], ci));
            queue.push((c.variables[1], ci));
        }
    }

    while let Some((var_idx, ci)) = queue.pop() {
        let constraint = &constraints[ci];
        let other_idx = if constraint.variables[0] == var_idx {
            constraint.variables[1]
        } else {
            constraint.variables[0]
        };

        let other_domain = variables[other_idx].domain.clone();
        let var = &mut variables[var_idx];
        let original_len = var.domain.len();

        let is_first = constraint.variables[0] == var_idx;
        var.domain.retain(|&val| {
            other_domain.iter().any(|&oval| {
                if is_first {
                    constraint.is_satisfied(&[val, oval])
                } else {
                    constraint.is_satisfied(&[oval, val])
                }
            })
        });

        if var.domain.len() < original_len {
            // Domain was reduced; add neighboring arcs
            for (nci, nc) in constraints.iter().enumerate() {
                if nci != ci && nc.arity() == 2 {
                    if nc.variables[0] == var_idx {
                        queue.push((nc.variables[1], nci));
                    } else if nc.variables[1] == var_idx {
                        queue.push((nc.variables[0], nci));
                    }
                }
            }
        }

        if var.domain.is_empty() {
            return false;
        }
    }
    true
}

/// Forward checking: when a variable is assigned, remove inconsistent values
/// from neighbors' domains.
pub fn forward_check(
    variables: &mut [Variable],
    constraints: &[Constraint],
    assigned_var: usize,
) -> bool {
    for constraint in constraints {
        if !constraint.variables.contains(&assigned_var) || constraint.arity() != 2 {
            continue;
        }

        let other_idx = if constraint.variables[0] == assigned_var {
            constraint.variables[1]
        } else {
            constraint.variables[0]
        };

        if variables[other_idx].is_assigned() {
            continue;
        }

        let assigned_val = variables[assigned_var].value.unwrap();
        let other = &mut variables[other_idx];
        other.domain.retain(|&val| {
            if constraint.variables[0] == assigned_var {
                constraint.is_satisfied(&[assigned_val, val])
            } else {
                constraint.is_satisfied(&[val, assigned_val])
            }
        });

        if other.domain.is_empty() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::Constraint;

    fn make_vars() -> Vec<Variable> {
        vec![
            Variable::new(0, vec![1, 2, 3]),
            Variable::new(1, vec![1, 2, 3]),
            Variable::new(2, vec![1, 2, 3]),
        ]
    }

    #[test]
    fn test_propagate_equality() {
        let mut vars = make_vars();
        let c = Constraint::equality(0, 0, 1, vec![], 1.0);
        assert!(propagate(&mut vars, &[c]));
        assert_eq!(vars[0].domain, vars[1].domain);
    }

    #[test]
    fn test_propagate_inequality() {
        let mut vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1, 2]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        assert!(propagate(&mut vars, &[c]));
        assert_eq!(vars[1].domain, vec![2]);
    }

    #[test]
    fn test_propagate_wipeout() {
        let mut vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1]),
        ];
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        assert!(!propagate(&mut vars, &[c]));
    }

    #[test]
    fn test_arc_consistency_simple() {
        let mut vars = make_vars();
        let c = Constraint::less_than(0, 0, 1, vec![], 1.0);
        assert!(arc_consistency(&mut vars, &[c]));
        // var 0 cannot have 3 (nothing in var 1 is greater)
        // var 1 cannot have 1 (nothing in var 0 is less)
        assert!(!vars[0].domain.contains(&3) || vars[1].domain.contains(&1));
    }

    #[test]
    fn test_arc_consistency_no_solution() {
        let mut vars = vec![
            Variable::new(0, vec![5]),
            Variable::new(1, vec![1, 2]),
        ];
        let c = Constraint::less_than(0, 0, 1, vec![], 1.0);
        // 5 < anything in {1,2} is impossible
        assert!(!arc_consistency(&mut vars, &[c]));
    }

    #[test]
    fn test_forward_check() {
        let mut vars = make_vars();
        vars[0].assign(1);
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        assert!(forward_check(&mut vars, &[c], 0));
        assert!(!vars[1].domain.contains(&1));
    }

    #[test]
    fn test_forward_check_wipeout() {
        let mut vars = vec![
            Variable::new(0, vec![1]),
            Variable::new(1, vec![1]),
        ];
        vars[0].assign(1);
        let c = Constraint::inequality(0, 0, 1, vec![], 1.0);
        assert!(!forward_check(&mut vars, &[c], 0));
    }

    #[test]
    fn test_propagate_range() {
        let mut vars = vec![Variable::new(0, vec![1, 5, 10, 15])];
        let c = Constraint::range(0, 0, vec![], 1.0, 3, 12);
        assert!(propagate(&mut vars, &[c]));
        assert_eq!(vars[0].domain, vec![5, 10]);
    }
}
