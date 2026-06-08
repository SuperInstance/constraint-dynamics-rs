# constraint-dynamics-rs

Constraint dynamics — how constraints shape agent behavior over time. Provides constraint satisfaction problem (CSP) modeling, backtracking search with forward checking, arc consistency propagation, constraint relaxation for over-constrained systems, and energy landscape analysis.

## What It Does

Agents in a fleet operate under constraints: resource limits, dependency ordering, conflict avoidance, conservation budgets. This crate models those constraints explicitly, solves for valid assignments, and when no solution exists, relaxes constraints to find the best partial solution.

Core modules:

- **`constraint`** — `Constraint`, `Variable` types with equality, inequality, less-than, range, and custom predicates
- **`dynamics`** — Constraint propagation, arc consistency (AC-3), forward checking
- **`solver`** — Backtracking solver with MRV heuristic and forward checking
- **`relaxation`** — Relaxation strategies for over-constrained systems
- **`energy`** — Energy landscape analysis: minima, barriers, ruggedness

## Quick Start

```toml
[dependencies]
constraint-dynamics-rs = { git = "https://github.com/SuperInstance/constraint-dynamics-rs" }
```

```rust
use constraint_dynamics_rs::constraint::{Constraint, Variable};
use constraint_dynamics_rs::solver::Solver;

fn main() {
    // 3 variables, domains {1, 2, 3}
    let vars = vec![
        Variable::new(0, vec![1, 2, 3]),
        Variable::new(1, vec![1, 2, 3]),
        Variable::new(2, vec![1, 2, 3]),
    ];

    // All must be different (graph coloring K3)
    let constraints = vec![
        Constraint::inequality(0, 0, 1, vec![], 1.0),
        Constraint::inequality(1, 1, 2, vec![], 1.0),
        Constraint::inequality(2, 0, 2, vec![], 1.0),
    ];

    let solver = Solver::new();
    let solution = solver.solve(&vars, &constraints).unwrap();

    for (var_id, value) in &solution.assignments {
        println!("Variable {} = {}", var_id, value);
    }
    println!("Backtracks: {}", solution.backtracks);
}
```

## Constraints

### Built-in Constraint Types

```rust
use constraint_dynamics_rs::constraint::Constraint;

// Equality: var_a == var_b
let eq = Constraint::equality(0, 0, 1, vec![vec![1, 2], vec![1, 2]], 1.0);

// Inequality: var_a != var_b
let neq = Constraint::inequality(1, 0, 1, vec![vec![1, 2], vec![1, 2]], 1.0);

// Less than: var_a < var_b
let lt = Constraint::less_than(2, 0, 1, vec![vec![1, 2, 3], vec![1, 2, 3]], 1.0);

// Range: var must be in [lo, hi]
let range = Constraint::range(3, 0, vec![vec![1, 5, 10, 15]], 0.8, 3, 12);

// Custom predicate
let custom = Constraint::new(
    4,
    vec![0, 1],
    vec![vec![1, 2, 3, 4], vec![1, 2, 3, 4]],
    1.0,
    |vals| vals[0] + vals[1] == 5,
);
```

### Constraint Properties

```rust
let c = Constraint::equality(0, 0, 1, vec![], 1.0);

assert_eq!(c.arity(), 2);
assert!(c.is_mandatory());          // strength == 1.0
assert!(c.is_satisfied(&[2, 2]));
assert!(!c.is_satisfied(&[1, 2]));
println!("{}", c); // Constraint(id=0, vars=[0, 1], strength=1.00, type=Binary)
```

### Strength

Constraints have a strength from 0.0 (weak/preference) to 1.0 (mandatory). Weak constraints can be relaxed when the system is over-constrained.

```rust
// Mandatory (must be satisfied)
let mandatory = Constraint::inequality(0, 0, 1, vec![], 1.0);

// Preference (prefer to be satisfied, can be dropped)
let preference = Constraint::less_than(1, 0, 1, vec![], 0.3);
```

## Variables

```rust
use constraint_dynamics_rs::constraint::Variable;

let mut v = Variable::new(0, vec![1, 2, 3, 4, 5]);

v.assign(3);
assert!(v.is_assigned());
assert_eq!(v.value, Some(3));

v.unassign();
assert!(!v.is_assigned());

v.remove_from_domain(3);
assert_eq!(v.domain_size(), 4); // {1, 2, 4, 5}
```

## Solver

### Backtracking with Forward Checking

The solver uses the MRV (Minimum Remaining Values) heuristic to pick the most constrained variable first, and forward checking to prune domains early.

```rust
use constraint_dynamics_rs::constraint::{Constraint, Variable};
use constraint_dynamics_rs::solver::Solver;

let vars = vec![
    Variable::new(0, vec![1, 2, 3]),
    Variable::new(1, vec![1, 2, 3]),
    Variable::new(2, vec![1, 2, 3]),
];

let constraints = vec![
    Constraint::less_than(0, 0, 1, vec![], 1.0),
    Constraint::less_than(1, 1, 2, vec![], 1.0),
];

let solver = Solver::new(); // forward checking enabled by default
let sol = solver.solve(&vars, &constraints).unwrap();

let v0 = sol.assignments.iter().find(|(id, _)| *id == 0).unwrap().1;
let v1 = sol.assignments.iter().find(|(id, _)| *id == 1).unwrap().1;
let v2 = sol.assignments.iter().find(|(id, _)| *id == 2).unwrap().1;
assert!(v0 < v1);
assert!(v1 < v2);
```

### Without Forward Checking

```rust
let mut solver = Solver::new();
solver.use_forward_check = false;
```

### Limited Backtracks

```rust
let mut solver = Solver::new();
solver.max_backtracks = 1000; // give up after 1000 backtracks
```

### Count Solutions

```rust
let solver = Solver::new();
let count = solver.count_solutions(&vars, &constraints, 1000);
println!("Found {} solutions (capped at 1000)", count);
```

## Constraint Propagation

### Arc Consistency (AC-3)

Remove values from variable domains that cannot participate in any valid assignment.

```rust
use constraint_dynamics_rs::constraint::{Constraint, Variable};
use constraint_dynamics_rs::dynamics::arc_consistency;

let mut vars = vec![
    Variable::new(0, vec![1, 2, 3]),
    Variable::new(1, vec![1, 2, 3]),
];

let constraints = vec![
    Constraint::less_than(0, 0, 1, vec![], 1.0),
];

let consistent = arc_consistency(&mut vars, &constraints);
assert!(consistent);
// var 0: domain reduced (3 removed — nothing greater in var 1)
// var 1: domain reduced (1 removed — nothing less in var 0)
```

### General Propagation

Works with unary, binary, and n-ary constraints:

```rust
use constraint_dynamics_rs::dynamics::propagate;

let mut vars = vec![Variable::new(0, vec![1, 5, 10, 15])];
let constraints = vec![Constraint::range(0, 0, vec![], 1.0, 3, 12)];

assert!(propagate(&mut vars, &constraints));
assert_eq!(vars[0].domain, vec![5, 10]); // 1 and 15 removed
```

### Forward Checking

When a variable is assigned, remove inconsistent values from neighbors:

```rust
use constraint_dynamics_rs::dynamics::forward_check;

let mut vars = vec![
    Variable::new(0, vec![1, 2, 3]),
    Variable::new(1, vec![1, 2, 3]),
];
vars[0].assign(1);

let constraints = vec![Constraint::inequality(0, 0, 1, vec![], 1.0)];
assert!(forward_check(&mut vars, &constraints, 0));
// var 1 domain: {2, 3} — 1 removed because it would violate inequality
```

### Detecting No Solution

```rust
let mut vars = vec![
    Variable::new(0, vec![5]),
    Variable::new(1, vec![1, 2]),
];
let constraints = vec![Constraint::less_than(0, 0, 1, vec![], 1.0)];
// 5 < {1, 2} is impossible
assert!(!propagate(&mut vars, &constraints));
```

## Constraint Relaxation

When a CSP has no solution, relax constraints to find the best partial solution.

### Weakest-First Relaxation

Remove the weakest constraints until a solution is found:

```rust
use constraint_dynamics_rs::constraint::{Constraint, Variable};
use constraint_dynamics_rs::relaxation::{relaxed_solve, RelaxationStrategy};

let vars = vec![
    Variable::new(0, vec![1]),
    Variable::new(1, vec![1, 2]),
    Variable::new(2, vec![1, 2]),
];

let constraints = vec![
    Constraint::equality(0, 0, 1, vec![], 1.0),    // mandatory
    Constraint::inequality(1, 1, 2, vec![], 1.0),   // mandatory
    Constraint::equality(2, 0, 2, vec![], 0.5),     // weak — will be dropped
];

let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::WeakestFirst).unwrap();
println!("Satisfaction ratio: {:.1}%", result.satisfaction_ratio() * 100.0);
println!("Violated constraints: {:?}", result.violated);
```

### Remove Specific Constraint

```rust
let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::RemoveIndex(2)).unwrap();
```

### Reverse Order Relaxation

```rust
let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::ReverseOrder).unwrap();
```

### Reduce Strength

```rust
let result = relaxed_solve(&vars, &constraints, &RelaxationStrategy::ReduceStrength(0.5)).unwrap();
```

### Minimal Relaxation

Find the minimum set of constraints to relax:

```rust
use constraint_dynamics_rs::relaxation::minimal_relaxation;

let result = minimal_relaxation(&vars, &constraints);
if let Some(sol) = result {
    println!("Found solution by relaxing constraints: {:?}", sol.violated);
}
```

### RelaxedSolution Fields

```rust
pub struct RelaxedSolution {
    pub assignments: Vec<(usize, i64)>,    // variable assignments
    pub satisfied_weight: f64,               // total weight of satisfied constraints
    pub total_weight: f64,                   // total weight of all constraints
    pub violated: Vec<usize>,                // indices of violated constraints
}

impl RelaxedSolution {
    pub fn satisfaction_ratio(&self) -> f64; // satisfied_weight / total_weight
}
```

## Energy Landscape

Model the CSP as an energy landscape where violations increase energy. The goal is to find low-energy states.

### Compute Energy

```rust
use constraint_dynamics_rs::constraint::Constraint;
use constraint_dynamics_rs::energy::EnergyLandscape;

let domains = vec![vec![1, 2], vec![1, 2]];
let constraints = vec![Constraint::inequality(0, 0, 1, vec![], 1.0)];
let landscape = EnergyLandscape::new(domains, constraints);

println!("Energy of [1,2]: {:.1}", landscape.energy(&[1, 2])); // 0.0 (satisfied)
println!("Energy of [1,1]: {:.1}", landscape.energy(&[1, 1])); // 1.0 (violated)
```

### Find Global Minima

```rust
let minima = landscape.global_minima();
for (state, energy) in &minima {
    println!("State {:?}: energy = {:.1}", state, energy);
}
// [1, 2]: energy = 0.0
// [2, 1]: energy = 0.0
```

### Minimum Energy

```rust
println!("Minimum energy: {:.1}", landscape.minimum_energy()); // 0.0
```

### Local Minima

```rust
let local = landscape.local_minima();
println!("{} local minima found", local.len());
```

### Energy Barriers

```rust
let barrier = landscape.energy_barrier(&[1, 2], &[2, 1]);
println!("Energy barrier between [1,2] and [2,1]: {:.1}", barrier);
```

### Landscape Ruggedness

```rust
let ruggedness = landscape.ruggedness();
println!("Energy variance (ruggedness): {:.4}", ruggedness);
```

### Over-Constrained Landscape

```rust
let domains = vec![vec![1], vec![1]];
let constraints = vec![Constraint::inequality(0, 0, 1, vec![], 1.0)];
let landscape = EnergyLandscape::new(domains, constraints);

// Only state [1,1] exists and it violates the constraint
assert_eq!(landscape.minimum_energy(), 1.0); // no satisfying assignment
```

## Lagrangian Mechanics for Agents

The energy landscape module provides a natural Lagrangian formulation. The total energy (Hamiltonian) of an agent fleet under constraints is:

```
H = Σ constraint_violation_energy
```

The Lagrangian is:

```
L = T - V
```

Where `T` is the kinetic energy (agent movement/correction cost) and `V` is the potential energy (constraint violation). Agents naturally seek low-energy states — the `global_minima()` method finds these.

```rust
use constraint_dynamics_rs::constraint::Constraint;
use constraint_dynamics_rs::energy::EnergyLandscape;

// Model: 3 agents sharing a resource pool of 6 units
// Each agent uses 1-5 units
let domains = vec![vec![1, 2, 3, 4, 5]; 3];

// Conservation constraint: total usage must be ≤ 6
let conservation = Constraint::new(
    0,
    vec![0, 1, 2],
    domains.clone(),
    1.0,
    |vals| vals.iter().sum::<i64>() <= 6,
);

let landscape = EnergyLandscape::new(domains, vec![conservation]);

let min_energy = landscape.minimum_energy();
let minima = landscape.global_minima();

println!("Minimum energy: {:.1}", min_energy);
println!("Valid assignments (total ≤ 6):");
for (state, e) in &minima {
    if *e == 0.0 {
        println!("  {:?} (sum = {})", state, state.iter().sum::<i64>());
    }
}
```

## si-cli Integration

```bash
# Define constraints for agent fleet
si fleet constrain --type inequality --agents 0,1 --strength 1.0
si fleet constrain --type range --agent 2 --min 1 --max 5

# Solve
si fleet solve --method backtracking

# When over-constrained, relax
si fleet solve --method relaxed --strategy weakest-first

# Analyze landscape
si fleet landscape --analyze
```

## si-fleet-api Integration

```
POST /v1/fleet/constraints/solve
{
    "variables": [
        {"id": 0, "domain": [1, 2, 3]},
        {"id": 1, "domain": [1, 2, 3]}
    ],
    "constraints": [
        {"type": "inequality", "variables": [0, 1], "strength": 1.0}
    ]
}

→ {
    "assignments": [[0, 1], [1, 2]],
    "backtracks": 0,
    "solved": true
}
```

## Supabase Integration

```sql
CREATE TABLE fleet_constraints (
    fleet_id UUID REFERENCES fleets(id),
    constraint_id INT NOT NULL,
    constraint_type TEXT NOT NULL,  -- 'equality', 'inequality', 'less_than', 'range', 'custom'
    variables INT[] NOT NULL,
    strength FLOAT NOT NULL DEFAULT 1.0,
    params JSONB,  -- e.g. {"lo": 3, "hi": 12} for range constraints
    PRIMARY KEY (fleet_id, constraint_id)
);

CREATE TABLE fleet_solutions (
    fleet_id UUID REFERENCES fleets(id),
    assignments JSONB NOT NULL,   -- [[0, 1], [1, 2], [2, 3]]
    backtracks INT NOT NULL,
    violated_constraints INT[] DEFAULT '{}',
    satisfaction_ratio FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Architecture

```
src/
├── lib.rs           — Module declarations
├── constraint.rs    — Constraint, Variable, ConstraintType
├── dynamics.rs      — propagate, arc_consistency, forward_check
├── solver.rs        — Solver, Solution (backtracking + MRV + forward checking)
├── relaxation.rs    — relaxed_solve, minimal_relaxation, RelaxationStrategy
└── energy.rs        — EnergyLandscape (energy, minima, barriers, ruggedness)
```

## Testing

```bash
cargo test
```

Tests cover:
- Equality, inequality, less-than, range constraints
- Variable assignment and domain reduction
- Constraint propagation for unary, binary, n-ary constraints
- Arc consistency (AC-3)
- Forward checking
- Backtracking solver with MRV heuristic
- Solution counting
- Relaxation strategies (weakest-first, reverse, remove-index, reduce-strength)
- Energy landscapes: minima, barriers, ruggedness
- Over-constrained detection

## License

MIT
