#![forbid(unsafe_code)]

pub mod attention;
pub mod attention_evidence;
pub mod attention_page;
pub mod bme;
pub mod bme_cost;
pub mod bme_equation;
pub mod bme_packed;
pub mod circuit;
pub mod hybrid;
pub mod sparsity;
pub mod sparsity_dynamic;
pub mod sparsity_ranking;
pub mod sparsity_relational;
pub mod sparsity_static;
pub mod sparsity_structured;
pub mod state;

pub use attention::{
    AdmissionScore, AttentionRouterError, BitSignature, admit_by_hamming, hamming_admission_row,
    hamming_distance, score_admission,
};
pub use attention_evidence::{
    AttentionSystemsEvidence, AttentionTimingEvidence, AttentionWorkEvidence, SystemsEvidenceError,
    TimingEvidenceKind, TrafficEvidenceKind,
};
pub use attention_page::{
    PageEnvelope, PageEnvelopeError, admit_page_by_hamming_lower_bound, hamming_page_admission_row,
    page_hamming_lower_bound,
};
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
pub use sparsity_dynamic::{DynamicMaskError, dynamic_mask_from_predicates};
pub use sparsity_ranking::{
    StructuredSparsityError, deterministic_random_keys, deterministic_random_mask,
    mask_from_descending_u64_scores, rank_descending_u64, structured_nm_mask_from_u64_scores,
};
pub use sparsity_relational::{RedundancyEdge, RelationalMaskError, relational_component_mask};
pub use sparsity_static::{
    StaticMaskError, static_mask_from_index_bits, static_mask_from_truth_table,
};
pub use sparsity_structured::{StructuredBooleanMaskError, structured_group_mask_from_truth_table};
pub use state::{BitState, StateError};
