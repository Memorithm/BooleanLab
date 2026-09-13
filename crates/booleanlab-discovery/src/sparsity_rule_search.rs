//! BL-14.5 exact multi-objective screening for already-evaluated sparsity rules.
//!
//! This module does not synthesize rules by itself. It provides the common,
//! deterministic evidence boundary that SAT/MaxSAT, CEGIS, Forge-style search
//! or another declared discrete search method can feed after evaluating a
//! candidate under one frozen contract. Search evidence and final holdout
//! evidence are deliberately separate and may not be pooled into one frontier.

use std::collections::BTreeSet;
use std::fmt;

/// Evidence partition used by BL-14.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SparsityEvaluationPhase {
    /// Evidence that a search procedure may inspect while proposing/selecting rules.
    Search,
    /// Frozen final evidence that must not feed back into rule selection.
    Holdout,
}

/// Exact objective vector for one already-evaluated Boolean sparsity rule.
///
/// Every field is retained independently; this type intentionally does not
/// collapse the BL-14 objective into one scalar score. All objectives are
/// minimized when computing the Pareto frontier. `quality_loss_units` is signed
/// so a measured quality improvement relative to the declared reference may be
/// represented as a negative value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparsityRuleCandidate {
    /// Stable experiment-owned identifier for this rule/configuration.
    pub candidate_id: String,
    /// Search or frozen final holdout evidence.
    pub phase: SparsityEvaluationPhase,
    /// Task-quality loss relative to the declared reference in frozen integer units.
    pub quality_loss_units: i64,
    /// Number of retained groups/elements in the matched evaluation domain.
    pub retained_units: u64,
    /// Total groups/elements in that evaluation domain.
    pub total_units: u64,
    /// Exact declared Boolean-controller cost units.
    pub controller_cost_units: u64,
    /// Effective numerical-compute units consumed after sparsification.
    pub effective_compute_units: u64,
    /// Measured or explicitly modelled memory-traffic bytes under one frozen provenance.
    pub memory_traffic_bytes: u64,
    /// End-to-end latency in nanoseconds under one frozen timing protocol.
    pub latency_ns: u64,
    /// Exact declared rule-complexity units (for example literals or nodes).
    pub rule_complexity_units: u64,
}

/// Fail-closed validation errors for BL-14.5 candidate screening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparsityRuleSearchError {
    EmptyCandidates,
    EmptyCandidateId { index: usize },
    DuplicateCandidateId { candidate_id: String },
    ZeroTotalUnits { candidate_id: String },
    RetainedUnitsExceedTotal {
        candidate_id: String,
        retained_units: u64,
        total_units: u64,
    },
    MixedEvaluationPhase {
        expected: SparsityEvaluationPhase,
        actual: SparsityEvaluationPhase,
        candidate_id: String,
    },
    EvaluationDomainSizeMismatch {
        expected_total_units: u64,
        actual_total_units: u64,
        candidate_id: String,
    },
}

/// Return original candidate indices that are not Pareto-dominated.
///
/// Candidate order is preserved exactly. Equal objective vectors do not
/// dominate one another; provenance-distinct ties therefore remain visible to
/// later reporting rather than being silently deduplicated.
///
/// # Errors
///
/// Rejects an empty candidate population, empty/duplicate identifiers, invalid
/// retained cardinality, mixed SEARCH/HOLDOUT evidence, or candidates evaluated
/// over different total domain sizes.
pub fn pareto_frontier_indices(
    candidates: &[SparsityRuleCandidate],
) -> Result<Vec<usize>, SparsityRuleSearchError> {
    validate_candidates(candidates)?;

    Ok((0..candidates.len())
        .filter(|&candidate_index| {
            !(0..candidates.len()).any(|other_index| {
                other_index != candidate_index
                    && dominates(&candidates[other_index], &candidates[candidate_index])
            })
        })
        .collect())
}

fn validate_candidates(candidates: &[SparsityRuleCandidate]) -> Result<(), SparsityRuleSearchError> {
    let Some(first) = candidates.first() else {
        return Err(SparsityRuleSearchError::EmptyCandidates);
    };
    let expected_phase = first.phase;
    let expected_total_units = first.total_units;
    let mut candidate_ids = BTreeSet::new();

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.candidate_id.is_empty() {
            return Err(SparsityRuleSearchError::EmptyCandidateId { index });
        }
        if !candidate_ids.insert(candidate.candidate_id.as_str()) {
            return Err(SparsityRuleSearchError::DuplicateCandidateId {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        if candidate.total_units == 0 {
            return Err(SparsityRuleSearchError::ZeroTotalUnits {
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        if candidate.retained_units > candidate.total_units {
            return Err(SparsityRuleSearchError::RetainedUnitsExceedTotal {
                candidate_id: candidate.candidate_id.clone(),
                retained_units: candidate.retained_units,
                total_units: candidate.total_units,
            });
        }
        if candidate.phase != expected_phase {
            return Err(SparsityRuleSearchError::MixedEvaluationPhase {
                expected: expected_phase,
                actual: candidate.phase,
                candidate_id: candidate.candidate_id.clone(),
            });
        }
        if candidate.total_units != expected_total_units {
            return Err(SparsityRuleSearchError::EvaluationDomainSizeMismatch {
                expected_total_units,
                actual_total_units: candidate.total_units,
                candidate_id: candidate.candidate_id.clone(),
            });
        }
    }

    Ok(())
}

fn dominates(left: &SparsityRuleCandidate, right: &SparsityRuleCandidate) -> bool {
    let no_worse = left.quality_loss_units <= right.quality_loss_units
        && left.retained_units <= right.retained_units
        && left.controller_cost_units <= right.controller_cost_units
        && left.effective_compute_units <= right.effective_compute_units
        && left.memory_traffic_bytes <= right.memory_traffic_bytes
        && left.latency_ns <= right.latency_ns
        && left.rule_complexity_units <= right.rule_complexity_units;

    let strictly_better = left.quality_loss_units < right.quality_loss_units
        || left.retained_units < right.retained_units
        || left.controller_cost_units < right.controller_cost_units
        || left.effective_compute_units < right.effective_compute_units
        || left.memory_traffic_bytes < right.memory_traffic_bytes
        || left.latency_ns < right.latency_ns
        || left.rule_complexity_units < right.rule_complexity_units;

    no_worse && strictly_better
}

impl fmt::Display for SparsityRuleSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsityRuleSearchError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        candidate_id: &str,
        phase: SparsityEvaluationPhase,
        quality_loss_units: i64,
        retained_units: u64,
        controller_cost_units: u64,
    ) -> SparsityRuleCandidate {
        SparsityRuleCandidate {
            candidate_id: candidate_id.to_owned(),
            phase,
            quality_loss_units,
            retained_units,
            total_units: 100,
            controller_cost_units,
            effective_compute_units: retained_units * 10,
            memory_traffic_bytes: retained_units * 64,
            latency_ns: retained_units * 100 + controller_cost_units,
            rule_complexity_units: controller_cost_units,
        }
    }

    #[test]
    fn returns_exact_non_dominated_candidates_in_original_order() {
        let candidates = vec![
            candidate("quality", SparsityEvaluationPhase::Search, 0, 80, 5),
            candidate("sparse", SparsityEvaluationPhase::Search, 4, 40, 4),
            candidate("dominated", SparsityEvaluationPhase::Search, 6, 60, 8),
            candidate("compact", SparsityEvaluationPhase::Search, 2, 60, 1),
        ];

        assert_eq!(pareto_frontier_indices(&candidates).unwrap(), vec![0, 1, 3]);
    }

    #[test]
    fn equal_objective_vectors_remain_distinct_provenance() {
        let first = candidate("a", SparsityEvaluationPhase::Search, 2, 50, 3);
        let second = candidate("b", SparsityEvaluationPhase::Search, 2, 50, 3);
        assert_eq!(
            pareto_frontier_indices(&[first, second]).unwrap(),
            vec![0, 1]
        );
    }

    #[test]
    fn search_and_holdout_evidence_cannot_be_pooled() {
        let search = candidate("search", SparsityEvaluationPhase::Search, 1, 50, 2);
        let holdout = candidate("holdout", SparsityEvaluationPhase::Holdout, 1, 50, 2);
        assert_eq!(
            pareto_frontier_indices(&[search, holdout]),
            Err(SparsityRuleSearchError::MixedEvaluationPhase {
                expected: SparsityEvaluationPhase::Search,
                actual: SparsityEvaluationPhase::Holdout,
                candidate_id: "holdout".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_invalid_or_unmatched_evaluation_domains() {
        let mut invalid = candidate("invalid", SparsityEvaluationPhase::Search, 1, 101, 2);
        assert!(matches!(
            pareto_frontier_indices(&[invalid.clone()]),
            Err(SparsityRuleSearchError::RetainedUnitsExceedTotal { .. })
        ));

        invalid.retained_units = 50;
        let mut other = candidate("other", SparsityEvaluationPhase::Search, 2, 50, 3);
        other.total_units = 200;
        assert!(matches!(
            pareto_frontier_indices(&[invalid, other]),
            Err(SparsityRuleSearchError::EvaluationDomainSizeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_and_empty_ids() {
        let first = candidate("same", SparsityEvaluationPhase::Search, 1, 50, 2);
        let second = candidate("same", SparsityEvaluationPhase::Search, 2, 40, 3);
        assert!(matches!(
            pareto_frontier_indices(&[first, second]),
            Err(SparsityRuleSearchError::DuplicateCandidateId { .. })
        ));

        let empty = candidate("", SparsityEvaluationPhase::Search, 1, 50, 2);
        assert_eq!(
            pareto_frontier_indices(&[empty]),
            Err(SparsityRuleSearchError::EmptyCandidateId { index: 0 })
        );
    }
}
