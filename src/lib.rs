//! # constraint-dynamics-rs
//!
//! Constraint dynamics — how constraints shape agent behavior over time.
//!
//! Provides constraint satisfaction, propagation, arc consistency, backtracking
//! solving with constraint propagation, relaxation for over-constrained systems,
//! and energy landscape analysis.

pub mod constraint;
pub mod dynamics;
pub mod energy;
pub mod relaxation;
pub mod solver;
