# INTEGRATION.md — constraint-dynamics-rs

## Role in the SuperInstance Ecosystem

constraint-dynamics-rs provides **constraint satisfaction and energy-based propagation** for the broader agent-behavior layer. It models agent decisions as constraint satisfaction problems (CSPs) and offers backtracking solvers, forward checking, and energy-landscape relaxation for over-constrained systems.

## SuperInstance Integration Points

### 1. Agent Behavior Modeling
- Agents declare goals as `Constraint` objects (unary, binary, n-ary)
- `Solver` resolves goal conflicts via backtracking + MRV heuristic
- Over-constrained scenarios fall back to `EnergyLandscape.relaxed_solve()` for minimal-violation solutions

### 2. si-runtime-python — Conservation Budget Coupling
- `AgentBudget` (si-runtime-python) enforces γ + η = total
- constraint-dynamics-rs can model this as a hard unary constraint: `gamma + eta == total`
- When an agent requests a budget transfer, the solver validates it against active constraints before `Budget.transfer()` is called

### 3. creative-engine-rust — Regime-Aware Constraints
- `CreativeSystem` (creative-engine-rust) reports its `Regime` (FixedPoint / Periodic / Chaotic)
- constraint-dynamics-rs adjusts constraint strengths dynamically:
  - **FixedPoint**: tighten constraints (low creativity, high safety)
  - **Chaotic**: loosen constraints (high creativity, tolerate violations)
  - **Periodic**: balanced constraint weights

### 4. flux-hyperbolic-rs — Tradition-Aware Embedding Constraints
- `TraditionEmbedding` (flux-hyperbolic-rs) provides hyperbolic coordinates for musical traditions
- constraint-dynamics-rs can encode "tradition compatibility" as a distance constraint in Poincaré ball space:
  ```rust
  Constraint::binary(
      |a, b| poincare.distance(a, b) < threshold,
      strength = 0.8,
  )
  ```

### 5. si-cli — Audit Integration
- `si-cli audit` discovers `*.rs` files and checks for:
  - Missing `Constraint` documentation
  - Hard-coded constraint strengths (should be configurable)
  - Solver timeout configurations
- Audit results are logged to Supabase `fleet_events` if `SUPABASE_URL` is set

### 6. superinstance-live — Real-Time Constraint Host
- `ConstraintHost` (superinstance-live) loads constraint pipelines at transport ticks
- constraint-dynamics-rs solvers run inside `ConstraintPipeline.on_tick()` to resolve real-time musical constraints (e.g., voice-leading rules, rhythmic compatibility)

## Dial / Room / Snap Compatibility

| Primitive | Mapping |
|-----------|---------|
| **Dial**  | `Constraint.strength` ∈ [0, 1] — higher dial = stronger constraint enforcement |
| **Room**  | `Solver` instance scoped to a single agent's local constraint graph |
| **Snap**  | `EnergyLandscape.snap()` — force all variables to hard constraint satisfaction, zero tolerance |
| **Cascade**| Propagate constraint reductions from parent agent to children via `RelaxedSolution.shared_subgraph` |

## Energy Conservation

The `EnergyLandscape` computes total energy as the sum of violated constraint strengths. This maps directly to the SuperInstance conservation law:

```
E_landscape = Σ violated_strengths
E_conserved = γ + η = total_budget
```

When `E_landscape` exceeds a threshold, the solver requests an `η → γ` transfer from `si-runtime-python.AgentBudget` to fund additional constraint propagation.

## Quick Start

```rust
use constraint_dynamics::{Constraint, Solver, EnergyLandscape};

let c1 = Constraint::unary(|x| x > 0, strength=0.9);
let c2 = Constraint::binary(|a, b| a + b == 10, strength=0.7);
let mut solver = Solver::new().with_forward_checking(true);
let solution = solver.solve(&[c1, c2], domains);
```

## Tests

```bash
cargo test
```

All solver tests, energy landscape tests, and propagation invariant checks must pass.
