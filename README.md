# constraint-dynamics-rs

**Constraint dynamics — how constraints shape agent behavior over time.**

This crate implements a complete constraint satisfaction and optimization framework in Rust: define typed constraints (unary, binary, n-ary) with strength-weighted predicates, propagate them through arc consistency algorithms, solve CSPs via backtracking with forward checking and MRV heuristics, relax over-constrained systems with configurable strategies, and analyze the energy landscape shaped by constraints. With 38 tests covering propagation, solving, relaxation, and landscape analysis, it provides the machinery for any system where rules govern behavior.

## Why This Matters

Constraints are the grammar of intelligent behavior. An AGI system doesn't just optimize — it operates within bounds: safety constraints, resource limits, social norms, logical consistency requirements. This crate treats constraints as first-class objects with *strengths*, meaning some rules are mandatory while others are preferences. When a system is over-constrained (which real-world systems always are), the relaxation engine finds the best partial solution. The energy landscape view reveals the global structure: where are the local minima? How many satisfying assignments exist? This is the mathematical framework for agents that must respect rules while remaining flexible.

## Quick Start

```toml
# Cargo.toml
[dependencies]
constraint-dynamics-rs = "0.1.0"
```

```rust
use constraint_dynamics_rs::constraint::{Constraint, Variable};
use constraint_dynamics_rs::solver::Solver;
use constraint_dynamics_rs::dynamics::propagate;
use constraint_dynamics_rs::relaxation::{relaxed_solve, RelaxationStrategy};
use constraint_dynamics_rs::energy::EnergyLandscape;

// Define variables with domains
let vars = vec![
    Variable::new(0, vec![1, 2, 3]),
    Variable::new(1, vec![1, 2, 3]),
    Variable::new(2, vec![1, 2, 3]),
];

// Binary constraint: x₀ ≠ x₁ (mandatory, strength = 1.0)
let neq_01 = Constraint::new(
    0, vec![0, 1],
    vec![vec![1, 2, 3], vec![1, 2, 3]],
    1.0,
    |vals| vals[0] != vals[1],
);

// Binary constraint: x₁ ≠ x₂
let neq_12 = Constraint::new(
    1, vec![1, 2],
    vec![vec![1, 2, 3], vec![1, 2, 3]],
    1.0,
    |vals| vals[0] != vals[1],
);

// Binary constraint: x₀ ≠ x₂
let neq_02 = Constraint::new(
    2, vec![0, 2],
    vec![vec![1, 2, 3], vec![1, 2, 3]],
    1.0,
    |vals| vals[0] != vals[1],
);

// Solve (graph coloring with 3 colors)
let solver = Solver::new();
if let Some(solution) = solver.solve(&vars, &[neq_01, neq_12, neq_02]) {
    println!("Solution: {:?}", solution.assignments);
    println!("Backtracks: {}", solution.backtracks);
}

// For over-constrained systems, relax with weakest-first strategy
let relaxed = relaxed_solve(&vars, &constraints, &RelaxationStrategy::WeakestFirst);
if let Some(sol) = relaxed {
    println!("Satisfaction: {:.1}%", sol.satisfaction_ratio() * 100.0);
}
```

## Architecture

| Module | Purpose |
|---|---|
| `constraint` | Core types: `Constraint`, `Variable`, `ConstraintType`, strength-weighted predicates |
| `dynamics` | Arc consistency, constraint propagation, forward checking |
| `solver` | Backtracking search with MRV heuristic and optional forward checking |
| `relaxation` | Over-constrained system solving: weakest-first, reverse-order, strength reduction |
| `energy` | Energy landscape analysis, global minima, brute-force enumeration |

## API Tour

### Core Types (`constraint`)

- **`Variable { id, domain, value }`** — A CSP variable
  - `::new(id, domain)` — Create unassigned
  - `.assign(val)`, `.unassign()` — Set/clear value
  - `.is_assigned()` — Check if bound
- **`Constraint { id, constraint_type, strength, variables, domain }`** — A constraint
  - `::new(id, variables, domain, strength, predicate)` — Full constructor
  - `.is_satisfied(assignment)` — Test the predicate
  - `.strength` — Weight (0.0 = weak preference, 1.0 = mandatory)
- **`ConstraintType`** — `Unary`, `Binary`, `Nary(usize)`

### Propagation (`dynamics`)

- **`propagate(variables, constraints) → bool`** — Full arc consistency
  - Returns `false` if any domain is wiped out (unsatisfiable)
- **`forward_check(variables, constraints, assigned_idx) → bool`** — Prune after assignment

### Solver (`solver`)

- **`Solver { use_forward_check, max_backtracks }`**
  - `::new()` — Default: forward checking enabled, unlimited backtracks
  - `.solve(variables, constraints) → Option<Solution>`
- **`Solution { assignments, backtracks }`** — Result with (var_id, value) pairs

### Relaxation (`relaxation`)

- **`RelaxationStrategy`** — `WeakestFirst`, `ReverseOrder`, `RemoveIndex(i)`, `ReduceStrength(factor)`
- **`relaxed_solve(variables, constraints, strategy) → Option<RelaxedSolution>`**
- **`RelaxedSolution { assignments, satisfied_weight, total_weight, violated }`**
  - `.satisfaction_ratio()` — Fraction of constraint weight satisfied

### Energy Landscape (`energy`)

- **`EnergyLandscape { num_variables, constraints, domains }`**
  - `.energy(assignment) → f64` — Energy of any complete assignment (0 = fully satisfied)
  - `.minimum_energy() → f64` — Brute-force global minimum
  - `.global_minima() → Vec<(Vec<i64>, f64)>` — All minimum-energy states

## Performance

- Propagation: O(n × constraints × domain_size) per iteration
- Backtracking: Exponential worst case, MRV heuristic dramatically reduces search space
- Forward checking: Cuts branches early, often 10-100× speedup
- Energy landscape: O(∏ domain_sizes) — brute-force, practical for ≤ ~10 variables with small domains
- Relaxation: O(n × solve_attempts) — removes constraints incrementally

## Ecosystem

Part of the **SuperInstance** family:

- [`sheaf-coherence-rs`](https://github.com/SuperInstance/sheaf-coherence-rs) — Consistency across distributed constraints
- [`agent-homeostasis-rs`](https://github.com/SuperInstance/agent-homeostasis-rs) — Homeostatic setpoints as dynamic constraints
- [`renormalization-group-rs`](https://github.com/SuperInstance/renormalization-group-rs) — Scale-dependent constraint analysis
- [`optimal-transport-rs`](https://github.com/SuperInstance/optimal-transport-rs) — Optimal reallocation under constraints

## Ideas for Improvement

- **Constraint learning** — Conflict-driven clause learning (CDCL) for CSP
- **Parallel solving** — Rayon-based branch parallelism
- **Symmetry breaking** — Automatic detection of symmetric variables
- **Soft constraints** — Continuous strength modeling with penalty functions
- **Incremental solving** — Efficient re-solve when constraints change
- **SAT/CSP bridge** — Integration with external SAT solvers via `cadical` or `kissat`

## License

MIT
