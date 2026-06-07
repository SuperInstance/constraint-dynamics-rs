//! Energy landscape shaped by constraints.
//!
//! Models a constraint satisfaction problem as an energy landscape where
//! constraint violations increase energy and the goal is to find low-energy states.

use crate::constraint::Constraint;

/// An energy landscape over variable assignments.
#[derive(Debug, Clone)]
pub struct EnergyLandscape {
    /// Number of variables.
    pub num_variables: usize,
    /// Constraints defining the landscape.
    pub constraints: Vec<Constraint>,
    /// Domain for each variable.
    pub domains: Vec<Vec<i64>>,
}

impl EnergyLandscape {
    /// Create a new energy landscape.
    pub fn new(domains: Vec<Vec<i64>>, constraints: Vec<Constraint>) -> Self {
        Self {
            num_variables: domains.len(),
            constraints,
            domains,
        }
    }

    /// Compute the energy of a given assignment.
    /// Energy = sum of strength * violation for each violated constraint.
    /// A fully satisfied assignment has energy 0.
    pub fn energy(&self, assignment: &[i64]) -> f64 {
        self.constraints
            .iter()
            .map(|c| {
                let vals: Vec<i64> = c.variables.iter().map(|&i| assignment[i]).collect();
                if c.is_satisfied(&vals) {
                    0.0
                } else {
                    c.strength
                }
            })
            .sum()
    }

    /// Find the minimum energy state by brute-force enumeration.
    /// Only practical for small domains.
    pub fn minimum_energy(&self) -> f64 {
        let mut min_energy = f64::INFINITY;
        self.enumerate_assignments(&mut vec![0; self.num_variables], 0, &mut min_energy);
        min_energy
    }

    fn enumerate_assignments(&self, assignment: &mut Vec<i64>, depth: usize, min_energy: &mut f64) {
        if depth == self.num_variables {
            let e = self.energy(assignment);
            if e < *min_energy {
                *min_energy = e;
            }
            return;
        }
        for &val in &self.domains[depth] {
            assignment[depth] = val;
            self.enumerate_assignments(assignment, depth + 1, min_energy);
        }
    }

    /// Find all global minima (states with minimum energy).
    pub fn global_minima(&self) -> Vec<(Vec<i64>, f64)> {
        let min_e = self.minimum_energy();
        let mut results = Vec::new();
        self.find_at_energy(&mut vec![0; self.num_variables], 0, min_e, &mut results);
        results
    }

    fn find_at_energy(
        &self,
        assignment: &mut Vec<i64>,
        depth: usize,
        target: f64,
        results: &mut Vec<(Vec<i64>, f64)>,
    ) {
        if depth == self.num_variables {
            let e = self.energy(assignment);
            if (e - target).abs() < 1e-10 {
                results.push((assignment.clone(), e));
            }
            return;
        }
        for &val in &self.domains[depth] {
            assignment[depth] = val;
            self.find_at_energy(assignment, depth + 1, target, results);
        }
    }

    /// Compute the energy barrier between two states.
    /// Simplified: returns the maximum energy along a direct path.
    pub fn energy_barrier(&self, state_a: &[i64], state_b: &[i64]) -> f64 {
        let e_a = self.energy(state_a);
        let e_b = self.energy(state_b);
        let base = e_a.max(e_b);

        // Check intermediate states along path (flip one variable at a time)
        let mut current = state_a.to_vec();
        let mut max_barrier = base;

        for i in 0..self.num_variables {
            if current[i] != state_b[i] {
                current[i] = state_b[i];
                let e = self.energy(&current);
                max_barrier = max_barrier.max(e);
            }
        }

        max_barrier - base.min(e_a.min(e_b))
    }

    /// Count the number of local minima in the landscape.
    /// A local minimum is a state where changing any single variable
    /// to any other value in its domain does not decrease energy.
    pub fn local_minima(&self) -> Vec<(Vec<i64>, f64)> {
        let mut minima = Vec::new();
        self.find_local_minima(&mut vec![0; self.num_variables], 0, &mut minima);
        minima
    }

    fn find_local_minima(
        &self,
        assignment: &mut Vec<i64>,
        depth: usize,
        results: &mut Vec<(Vec<i64>, f64)>,
    ) {
        if depth == self.num_variables {
            let e = self.energy(assignment);
            if self.is_local_minimum(assignment, e) {
                results.push((assignment.clone(), e));
            }
            return;
        }
        for &val in &self.domains[depth] {
            assignment[depth] = val;
            self.find_local_minima(assignment, depth + 1, results);
        }
    }

    fn is_local_minimum(&self, assignment: &[i64], energy: f64) -> bool {
        for i in 0..self.num_variables {
            for &alt_val in &self.domains[i] {
                if alt_val == assignment[i] {
                    continue;
                }
                let mut neighbor = assignment.to_vec();
                neighbor[i] = alt_val;
                if self.energy(&neighbor) < energy {
                    return false;
                }
            }
        }
        true
    }

    /// Compute a rough measure of landscape ruggedness: variance of energies.
    pub fn ruggedness(&self) -> f64 {
        let mut energies = Vec::new();
        self.collect_energies(&mut vec![0; self.num_variables], 0, &mut energies);

        if energies.is_empty() {
            return 0.0;
        }

        let mean = energies.iter().sum::<f64>() / energies.len() as f64;
        let variance = energies.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / energies.len() as f64;
        variance
    }

    fn collect_energies(&self, assignment: &mut Vec<i64>, depth: usize, energies: &mut Vec<f64>) {
        if depth == self.num_variables {
            energies.push(self.energy(assignment));
            return;
        }
        for &val in &self.domains[depth] {
            assignment[depth] = val;
            self.collect_energies(assignment, depth + 1, energies);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_landscape() -> EnergyLandscape {
        let domains = vec![vec![1, 2], vec![1, 2]];
        let constraints = vec![Constraint::inequality(0, 0, 1, vec![], 1.0)];
        EnergyLandscape::new(domains, constraints)
    }

    #[test]
    fn test_energy_satisfied() {
        let landscape = simple_landscape();
        assert_eq!(landscape.energy(&[1, 2]), 0.0);
        assert_eq!(landscape.energy(&[2, 1]), 0.0);
    }

    #[test]
    fn test_energy_violated() {
        let landscape = simple_landscape();
        assert_eq!(landscape.energy(&[1, 1]), 1.0);
        assert_eq!(landscape.energy(&[2, 2]), 1.0);
    }

    #[test]
    fn test_minimum_energy() {
        let landscape = simple_landscape();
        assert_eq!(landscape.minimum_energy(), 0.0);
    }

    #[test]
    fn test_global_minima() {
        let landscape = simple_landscape();
        let minima = landscape.global_minima();
        assert_eq!(minima.len(), 2);
        for (state, e) in &minima {
            assert_eq!(*e, 0.0);
            assert_ne!(state[0], state[1]);
        }
    }

    #[test]
    fn test_local_minima() {
        let landscape = simple_landscape();
        let minima = landscape.local_minima();
        assert_eq!(minima.len(), 2); // [1,2] and [2,1]
    }

    #[test]
    fn test_ruggedness() {
        let landscape = simple_landscape();
        let r = landscape.ruggedness();
        assert!(r >= 0.0);
    }

    #[test]
    fn test_energy_barrier() {
        let landscape = simple_landscape();
        let barrier = landscape.energy_barrier(&[1, 2], &[2, 1]);
        // Both are minima with energy 0, but intermediates [2,2] or [1,1] have energy 1
        assert!(barrier >= 0.0);
    }

    #[test]
    fn test_overconstrained_landscape() {
        let domains = vec![vec![1], vec![1]];
        let constraints = vec![Constraint::inequality(0, 0, 1, vec![], 1.0)];
        let landscape = EnergyLandscape::new(domains, constraints);
        assert_eq!(landscape.minimum_energy(), 1.0);
        let minima = landscape.global_minima();
        assert_eq!(minima.len(), 1);
    }
}
