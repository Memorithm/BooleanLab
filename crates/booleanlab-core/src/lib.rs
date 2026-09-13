#![forbid(unsafe_code)]

pub mod bme;
pub mod bme_cost;
pub mod bme_equation;
pub mod bme_packed;
pub mod circuit;
pub mod hybrid;
pub mod sparsity;
pub mod state;

pub use bme::{BmeError, or_and_cell, thresholded_xnor_cell, xnor_popcount_cell, xor_and_cell};
pub use bme_cost::{BmeCostError, BmeLogicalCost, BmeShape, logical_cost};
pub use bme_equation::{CanonicalBmeEquation, CanonicalBmeOutput};
pub use bme_packed::{
    PackedBmeError, packed_or_and_cell, packed_thresholded_xnor_cell, packed_xnor_popcount_cell,
    packed_xor_and_cell,
};
pub use circuit::{BooleanCircuit, CircuitError, Node, NodeId, TruthRow};
pub use hybrid::{HybridOperator, PredicateBridge};
pub use sparsity::{ExactMask, MaskCardinality, SparsityError};
pub use state::{BitState, StateError};
