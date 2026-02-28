use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Positive-Negative Counter CRDT.
/// Each node tracks its own increment and decrement totals.
/// Merge takes the max per node_id across both maps.
/// Properties: commutative, associative, idempotent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnCounter {
    pub node_id: String,
    increments: HashMap<String, u64>,
    decrements: HashMap<String, u64>,
}

impl PnCounter {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_string(),
            increments: HashMap::new(),
            decrements: HashMap::new(),
        }
    }

    pub fn increment(&mut self, amount: u64) {
        *self.increments.entry(self.node_id.clone()).or_insert(0) += amount;
    }

    pub fn decrement(&mut self, amount: u64) {
        *self.decrements.entry(self.node_id.clone()).or_insert(0) += amount;
    }

    pub fn value(&self) -> i64 {
        let pos: u64 = self.increments.values().sum();
        let neg: u64 = self.decrements.values().sum();
        pos as i64 - neg as i64
    }

    pub fn merge(&mut self, other: &PnCounter) {
        for (k, v) in &other.increments {
            let entry = self.increments.entry(k.clone()).or_insert(0);
            *entry = (*entry).max(*v);
        }
        for (k, v) in &other.decrements {
            let entry = self.decrements.entry(k.clone()).or_insert(0);
            *entry = (*entry).max(*v);
        }
    }
}
