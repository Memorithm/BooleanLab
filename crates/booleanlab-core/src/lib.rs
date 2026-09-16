#![forbid(unsafe_code)]

pub mod attention;
pub mod attention_evidence;
pub mod attention_page;
pub mod bme;
pub mod bme_cost;
pub mod bme_equation;
pub mod bme_packed;
pub mod bme_packed_work;
pub mod circuit;
pub mod hybrid;
pub mod kleene;
pub mod kleene_analysis;
pub mod kleene_conjunction;
pub mod kleene_conjunction_words;
pub mod kleene_designated_relation;
pub mod kleene_designated_subsumption;
pub mod kleene_entailment;
pub mod kleene_semantic_key;
pub mod pseudo_boolean;
pub mod pseudo_boolean_conjunction_entailment;
pub mod pseudo_boolean_entailment;
pub mod pseudo_boolean_normalization;
pub mod pseudo_boolean_relation;
pub mod sparsity;
pub mod sparsity_dynamic;
pub mod sparsity_ranking;
pub mod sparsity_relational;
pub mod sparsity_static;
pub mod sparsity_structured;
pub mod sparsity_synthesis;
pub mod state;
pub mod vectorial_ccz_screen;
pub mod vectorial_component_correlation;
pub mod vectorial_component_weights;
pub mod vectorial_degree;
pub mod vectorial_differential_spectrum;
pub mod vectorial_metrics;
pub mod vectorial_walsh_spectrum;

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
    PackedBmeError, packed_or_and_cell, packed_or_and_product_rows_columns,
    packed_thresholded_xnor_cell, packed_thresholded_xnor_product_rows_columns,
    packed_xnor_popcount_cell, packed_xnor_popcount_product_rows_columns, packed_xor_and_cell,
    packed_xor_and_product_rows_columns,
};
pub use bme_packed_work::{PackedBmeWorkError, PackedBmeWorkEstimate, packed_bme_work_estimate};
pub use circuit::{BooleanCircuit, CircuitError, Node, NodeId, TruthRow};
pub use hybrid::{HybridOperator, PredicateBridge};
pub use kleene::{
    BinaryTruthRow, KLEENE_BINARY_TRUTH_TABLE, KLEENE_VALUES, KleeneEvalError, KleeneInstruction,
    KleeneValue, evaluate_kleene_program, exhaustive_binary_truth_table,
};
pub use kleene_analysis::{
    DEFAULT_KLEENE_ANALYSIS_MAX_INSTRUCTION_EVALUATIONS, KleeneAnalysisError,
    KleeneComparisonError, KleeneProgramAnalysis, KleeneProgramComparison, KleeneProgramMismatch,
    analyze_kleene_program, analyze_kleene_program_with_work_budget, compare_kleene_programs,
};
pub use kleene_conjunction::{
    CompiledKleeneConjunction, KleeneConjunctionComparison, KleeneConjunctionComparisonError,
    KleeneConjunctionError, KleeneConjunctionMismatch, KleeneLiteral, MAX_COMPILED_KLEENE_INPUTS,
    compare_compiled_conjunction_with_program,
};
pub use kleene_conjunction_words::{
    CompiledMultiwordKleeneConjunction, MAX_MULTIWORD_KLEENE_INPUTS,
    MultiwordKleeneConjunctionError,
};
pub use kleene_designated_relation::{
    KleeneDesignatedDifferenceWitness, KleeneDesignatedRelation, kleene_designated_relation,
};
pub use kleene_designated_subsumption::{
    DEFAULT_MAX_DESIGNATED_SUBSUMPTION_PAIRS, KleeneDesignatedSubsumptionEdge,
    KleeneDesignatedSubsumptionError, KleeneDesignatedSubsumptionGraph,
    KleeneStrictSubsumptionWitness, kleene_designated_subsumption_graph,
};
pub use kleene_entailment::{
    KleeneEntailment, KleeneEntailmentError, KleeneEntailmentSide, KleeneEntailmentWitness,
    kleene_designated_entails,
};
pub use kleene_semantic_key::{
    KLEENE_SEMANTIC_KEY_SCHEMA_VERSION, KleeneSemanticKey, KleeneSemanticKeyError,
    kleene_semantic_key,
};
pub use pseudo_boolean::{
    DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS, MAX_PSEUDO_BOOLEAN_TERMS, PseudoBooleanConstraint,
    PseudoBooleanError, PseudoBooleanRelation, pseudo_boolean_equivalent,
    pseudo_boolean_equivalent_with_work_limit,
};
pub use pseudo_boolean_conjunction_entailment::{
    DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS, PseudoBooleanConjunctionError,
    PseudoBooleanConjunctionImplication, PseudoBooleanConjunctionWitness,
    pseudo_boolean_conjunction_implies, pseudo_boolean_conjunction_implies_with_work_limit,
};
pub use pseudo_boolean_entailment::{
    PseudoBooleanImplication, PseudoBooleanImplicationWitness, pseudo_boolean_implies,
    pseudo_boolean_implies_with_work_limit,
};
pub use pseudo_boolean_normalization::{
    primitive_pseudo_boolean_constraint, pseudo_boolean_common_factor,
};
pub use pseudo_boolean_relation::{
    PseudoBooleanDifferenceWitness, PseudoBooleanSetRelation, pseudo_boolean_set_relation,
    pseudo_boolean_set_relation_with_work_limit,
};
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
pub use sparsity_synthesis::{
    ConjunctiveSparsityRule, MAX_SYNTHESIS_PREDICATES, RuleSynthesisError, SparsityLiteral,
    SynthesisRow, synthesize_exact_conjunction,
};
pub use state::{BitState, StateError};
pub use vectorial_ccz_screen::{
    VectorialCczMismatch, VectorialCczScreen, VectorialCczScreenError,
    vectorial_ccz_invariant_screen, vectorial_ccz_invariant_screen_with_work_limit,
};
pub use vectorial_component_correlation::{
    VectorialComponentCorrelationProfile, vectorial_component_correlation_profile,
    vectorial_component_correlation_profile_with_work_limit,
};
pub use vectorial_component_weights::{
    VectorialComponentWeightSpectrum, vectorial_component_weight_spectrum,
    vectorial_component_weight_spectrum_with_work_limit,
};
pub use vectorial_degree::{
    DEFAULT_VECTORIAL_DEGREE_MAX_WORK, MAX_VECTORIAL_DEGREE_INPUT_BITS,
    MAX_VECTORIAL_DEGREE_OUTPUT_BITS, VectorialDegreeError, VectorialDegreeProfile,
    VectorialDegreeWitness, vectorial_degree_profile, vectorial_degree_profile_with_work_limit,
};
pub use vectorial_differential_spectrum::{
    VectorialDifferentialSpectrum, vectorial_differential_spectrum,
    vectorial_differential_spectrum_with_work_limit,
};
pub use vectorial_metrics::{
    DEFAULT_VECTORIAL_MAX_WORK, DifferentialUniformityWitness, MAX_VECTORIAL_INPUT_BITS,
    MAX_VECTORIAL_OUTPUT_BITS, VectorialBooleanMetrics, VectorialMetricsError,
    VectorialWalshWitness, vectorial_boolean_metrics, vectorial_boolean_metrics_with_work_limit,
};
pub use vectorial_walsh_spectrum::{
    VectorialWalshSpectrum, vectorial_walsh_spectrum, vectorial_walsh_spectrum_with_work_limit,
};
