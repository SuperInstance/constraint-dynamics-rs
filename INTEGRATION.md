# Integration Guide: constraint-dynamics-rs

## What This Crate Provides

Constraint dynamics — how constraints shape agent behavior over time. Provides constraint satisfaction, propagation, arc consistency, backtracking solving with constraint propagation, relaxation for over-constrained systems, and energy landscape analysis.

- **`constraint::Constraint`** — N-ary constraint with `id`, `constraint_type` (`Unary`/`Binary`/`Nary`), `strength` (0.0–1.0), `variables`, `domain`, and `predicate`. Methods: `new()`, `is_satisfied()`.
- **`constraint::Variable`** — CSP variable with `id`, `domain`, `value`, `is_assigned()`. Methods: `assign()`, `unassign()`.
- **`dynamics::propagate()`** — Iterative constraint propagation removing invalid domain values. Returns `true` if consistent, `false` if a domain is wiped out.
- **`dynamics::forward_check()`** — Forward-checking propagation after a variable assignment.
- **`dynamics::arc_consistency()`** — Enforces full arc consistency (AC-3) on the constraint graph.
- **`solver::Solver`** — Backtracking CSP solver with MRV heuristic and optional forward checking. Methods: `new()`, `solve()`, `backtrack()`, `select_variable()`, `is_consistent()`.
- **`solver::Solution`** — Result with `assignments` and `backtracks` count.
- **`energy::EnergyLandscape`** — Models a CSP as an energy landscape where violations increase energy. Methods: `new()`, `energy()`, `minimum_energy()`, `global_minima()`.
- **`relaxation::relaxed_solve()`** — Solve over-constrained systems by relaxing constraints using `WeakestFirst`, `ReverseOrder`, `RemoveIndex`, or `ReduceStrength` strategies.
- **`relaxation::RelaxedSolution`** — Partial solution with `satisfied_weight`, `total_weight`, `violated` indices, and `satisfaction_ratio()`.
- **`relaxation::RelaxationStrategy`** — Strategy enum for choosing which constraints to relax.

## How to Add This Crate

```bash
cargo add constraint-dynamics
```

```rust
use constraint_dynamics::{
    Constraint, Variable, Solver, EnergyLandscape,
    propagate, relaxed_solve, RelaxationStrategy,
};
```

## Cross-Repo Connections

### With `conservation-law-rs`: Energy Landscape as Entropy Reduction

Model constraint satisfaction as energy minimization, where the conservation law ensures total system energy never increases:

```rust
use constraint_dynamics::{Constraint, Variable, EnergyLandscape};
use conservation_law::lagrangian::{AgentState, total_energy};

fn solve_with_energy_bound(
    variables: &[Variable],
    constraints: &[Constraint],
    max_energy: f64,
) -> Option<Vec<(usize, i64)>> {
    let domains: Vec<Vec<i64>> = variables.iter().map(|v| v.domain.clone()).collect();
    let landscape = EnergyLandscape::new(domains, constraints.to_vec());
    
    // Brute-force search for low-energy states
    let mut best_energy = f64::INFINITY;
    let mut best_assignment = None;
    
    // Enumerate and check conservation bound
    let mut assignment = vec![0i64; variables.len()];
    enumerate(&landscape, &mut assignment, 0, max_energy, &mut best_energy, &mut best_assignment);
    
    best_assignment
}

fn enumerate(
    landscape: &EnergyLandscape,
    assignment: &mut [i64],
    depth: usize,
    max_energy: f64,
    best_energy: &mut f64,
    best_assignment: &mut Option<Vec<(usize, i64)>>,
) {
    if depth == landscape.num_variables {
        let e = landscape.energy(assignment);
        if e < *best_energy && e <= max_energy {
            *best_energy = e;
            *best_assignment = Some(assignment.iter().enumerate().map(|(i, &v)| (i, v)).collect());
        }
        return;
    }
    for &val in &landscape.domains[depth] {
        assignment[depth] = val;
        enumerate(landscape, assignment, depth + 1, max_energy, best_energy, best_assignment);
    }
}
```

### With `si-cli`: Interactive Constraint Solver

Expose the solver as a CLI command for fleet configuration validation:

```rust
use constraint_dynamics::{Solver, Variable, Constraint};

fn cli_solve_constraints(variables: Vec<Variable>, constraints: Vec<Constraint>) {
    let solver = Solver::new();
    
    match solver.solve(&variables, &constraints) {
        Some(solution) => {
            println!("Solution found in {} backtracks:", solution.backtracks);
            for (var_id, value) in &solution.assignments {
                println!("  Variable {} = {}", var_id, value);
            }
        }
        None => {
            println!("No solution exists. Try relaxing constraints:");
            let relaxed = relaxed_solve(&variables, &constraints, &RelaxationStrategy::WeakestFirst);
            if let Some(rs) = relaxed {
                println!("  Relaxed satisfaction: {:.1}%", rs.satisfaction_ratio() * 100.0);
                println!("  Violated constraints: {:?}", rs.violated);
            }
        }
    }
}
```

### With `si-fleet-api`: REST Constraint Validation

Validate fleet configurations against business rules via REST:

```rust
use constraint_dynamics::{Constraint, Variable, Solver, relaxed_solve, RelaxationStrategy};
use si_fleet_api::{HttpRequest, HttpResponse};

fn post_validate_config(req: HttpRequest) -> HttpResponse {
    let body: serde_json::Value = req.json().unwrap();
    let variables: Vec<Variable> = serde_json::from_value(body["variables"].clone()).unwrap();
    let constraints: Vec<Constraint> = serde_json::from_value(body["constraints"].clone()).unwrap();
    
    let solver = Solver::new();
    let result = solver.solve(&variables, &constraints);
    
    if let Some(solution) = result {
        HttpResponse::json(json!({
            "valid": true,
            "assignments": solution.assignments,
            "backtracks": solution.backtracks,
        }))
    } else {
        let relaxed = relaxed_solve(&variables, &constraints, &RelaxationStrategy::WeakestFirst);
        HttpResponse::json(json!({
            "valid": false,
            "relaxed": relaxed.map(|r| json!({
                "satisfaction_ratio": r.satisfaction_ratio(),
                "violated": r.violated,
                "assignments": r.assignments,
            })),
        }))
    }
}
```

### With Supabase: Persistent Constraint Problems

Store CSP instances and solutions in Supabase for audit and replay:

```rust
use constraint_dynamics::{Variable, Constraint, Solution};
use supabase_rs::SupabaseClient;

async fn persist_csp(
    client: &SupabaseClient,
    problem_id: &str,
    variables: &[Variable],
    constraints: &[Constraint],
) {
    client.from("csp_problems")
        .insert(json!({
            "problem_id": problem_id,
            "variables": serde_json::to_string(variables).unwrap(),
            "constraints": serde_json::to_string(constraints).unwrap(),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }))
        .execute()
        .await
        .unwrap();
}

async fn persist_solution(
    client: &SupabaseClient,
    problem_id: &str,
    solution: &Solution,
) {
    client.from("csp_solutions")
        .insert(json!({
            "problem_id": problem_id,
            "assignments": serde_json::to_string(&solution.assignments).unwrap(),
            "backtracks": solution.backtracks,
            "solved_at": chrono::Utc::now().to_rfc3339(),
        }))
        .execute()
        .await
        .unwrap();
}

async fn load_problem(client: &SupabaseClient, problem_id: &str) -> (Vec<Variable>, Vec<Constraint>) {
    let row = client.from("csp_problems")
        .select("*")
        .eq("problem_id", problem_id)
        .single()
        .execute()
        .await
        .unwrap();
    
    let variables: Vec<Variable> = serde_json::from_str(row.get("variables").unwrap()).unwrap();
    let constraints: Vec<Constraint> = serde_json::from_str(row.get("constraints").unwrap()).unwrap();
    (variables, constraints)
}
```

## Design Patterns

### Pattern: Incremental Constraint Propagation

Add constraints dynamically and re-propagate without restarting:

```rust
use constraint_dynamics::{Variable, Constraint, propagate};

fn incremental_add_constraint(
    variables: &mut [Variable],
    constraints: &mut Vec<Constraint>,
    new_constraint: Constraint,
) -> bool {
    constraints.push(new_constraint);
    propagate(variables, constraints)
}
```

### Pattern: Hierarchical Relaxation

Try increasingly aggressive relaxation strategies until a solution is found:

```rust
use constraint_dynamics::{relaxed_solve, RelaxationStrategy};

fn hierarchical_relax(variables: &[Variable], constraints: &[Constraint]) -> Option<RelaxedSolution> {
    for strategy in [
        RelaxationStrategy::ReduceStrength(0.8),
        RelaxationStrategy::WeakestFirst,
        RelaxationStrategy::ReverseOrder,
    ] {
        if let Some(sol) = relaxed_solve(variables, constraints, &strategy) {
            println!("Found solution with strategy: {:?}", strategy);
            return Some(sol);
        }
    }
    None
}
```

### Pattern: Energy-Guided Local Search

Use the energy landscape to guide greedy local search for near-optimal assignments:

```rust
use constraint_dynamics::EnergyLandscape;

fn energy_guided_search(landscape: &EnergyLandscape, max_iters: usize) -> (Vec<i64>, f64) {
    let mut assignment = vec![0i64; landscape.num_variables];
    let mut best = landscape.energy(&assignment);
    
    for _ in 0..max_iters {
        let mut improved = false;
        for i in 0..landscape.num_variables {
            for &val in &landscape.domains[i] {
                let mut trial = assignment.clone();
                trial[i] = val;
                let e = landscape.energy(&trial);
                if e < best {
                    best = e;
                    assignment = trial;
                    improved = true;
                }
            }
        }
        if !improved { break; }
    }
    
    (assignment, best)
}
```
