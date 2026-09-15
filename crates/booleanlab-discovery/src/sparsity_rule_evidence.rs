//! Provenance guard for BL-14.5 resource-aware Pareto screening.
//!
//! The underlying rule-search frontier stores exact objective values but cannot
//! by itself tell whether resource numbers came from a reference accounting
//! model or a hardware measurement, nor whether candidates used the same
//! protocol. This wrapper makes that distinction explicit and rejects mixed
//! evidence before a Pareto comparison is attempted. It is a SEARCH-only
//! screening surface: frozen HOLDOUT evidence is never accepted for selection.

use std::fmt;

use crate::sparsity_rule_search::{
    SparsityEvaluationPhase, SparsityRuleCandidate, SparsityRuleSearchError,
    pareto_frontier_indices,
};

/// Origin of resource objectives used by one BL-14.5 candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceEvidenceKind {
    /// Deterministic operation/traffic accounting under a declared reference model.
    ReferenceModel,
    /// Measurements collected under a declared hardware/timing protocol.
    Measured,
}

/// Provenance that makes resource-objective comparisons interpretable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceEvidenceProvenance {
    /// Whether the resource objectives are modelled or measured.
    pub kind: ResourceEvidenceKind,
    /// Stable identifier for the frozen accounting or measurement protocol.
    pub protocol_id: String,
    /// Immutable evidence/revision identifier used by that protocol.
    pub provenance_id: String,
}

/// One BL-14.5 candidate with explicit resource-evidence provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenancedSparsityRuleCandidate {
    pub candidate: SparsityRuleCandidate,
    pub resource_evidence: ResourceEvidenceProvenance,
}

/// Fail-closed errors for resource-provenance-aware screening.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparsityRuleEvidenceError {
    EmptyProtocolId {
        candidate_id: String,
    },
    EmptyProvenanceId {
        candidate_id: String,
    },
    HoldoutScreeningForbidden {
        candidate_id: String,
    },
    MixedResourceEvidenceKind {
        expected: ResourceEvidenceKind,
        actual: ResourceEvidenceKind,
        candidate_id: String,
    },
    ResourceProtocolMismatch {
        expected: String,
        actual: String,
        candidate_id: String,
    },
    ResourceProvenanceMismatch {
        expected: String,
        actual: String,
        candidate_id: String,
    },
    Candidate(SparsityRuleSearchError),
}

/// Compute a SEARCH Pareto frontier only after proving that resource objectives
/// are comparable under one declared evidence contract.
///
/// This function deliberately refuses to pool reference-model accounting with
/// hardware measurements, or measurements/accounting produced under different
/// protocol or provenance identifiers. It also rejects frozen HOLDOUT evidence:
/// final evidence may be reported after the selection is frozen, but it must not
/// feed back into this screening surface. The function does not turn a
/// reference-model byte, operation, or latency estimate into a measured hardware
/// result.
///
/// # Errors
///
/// Returns [`SparsityRuleEvidenceError`] when provenance is empty or mixed, when
/// HOLDOUT evidence is supplied for screening, or when the underlying BL-14.5
/// candidate validation fails.
pub fn pareto_frontier_indices_with_resource_provenance(
    candidates: &[ProvenancedSparsityRuleCandidate],
) -> Result<Vec<usize>, SparsityRuleEvidenceError> {
    let Some(first) = candidates.first() else {
        return pareto_frontier_indices(&[]).map_err(SparsityRuleEvidenceError::Candidate);
    };

    validate_provenance(first)?;
    if first.candidate.phase == SparsityEvaluationPhase::Holdout {
        return Err(SparsityRuleEvidenceError::HoldoutScreeningForbidden {
            candidate_id: first.candidate.candidate_id.clone(),
        });
    }
    let expected = &first.resource_evidence;

    for item in candidates.iter().skip(1) {
        validate_provenance(item)?;
        let actual = &item.resource_evidence;
        if actual.kind != expected.kind {
            return Err(SparsityRuleEvidenceError::MixedResourceEvidenceKind {
                expected: expected.kind,
                actual: actual.kind,
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
        if actual.protocol_id != expected.protocol_id {
            return Err(SparsityRuleEvidenceError::ResourceProtocolMismatch {
                expected: expected.protocol_id.clone(),
                actual: actual.protocol_id.clone(),
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
        if actual.provenance_id != expected.provenance_id {
            return Err(SparsityRuleEvidenceError::ResourceProvenanceMismatch {
                expected: expected.provenance_id.clone(),
                actual: actual.provenance_id.clone(),
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
    }

    let bare = candidates
        .iter()
        .map(|item| item.candidate.clone())
        .collect::<Vec<_>>();
    pareto_frontier_indices(&bare).map_err(SparsityRuleEvidenceError::Candidate)
}

fn validate_provenance(
    item: &ProvenancedSparsityRuleCandidate,
) -> Result<(), SparsityRuleEvidenceError> {
    if item.resource_evidence.protocol_id.trim().is_empty() {
        return Err(SparsityRuleEvidenceError::EmptyProtocolId {
            candidate_id: item.candidate.candidate_id.clone(),
        });
    }
    if item.resource_evidence.provenance_id.trim().is_empty() {
        return Err(SparsityRuleEvidenceError::EmptyProvenanceId {
            candidate_id: item.candidate.candidate_id.clone(),
        });
    }
    Ok(())
}

impl fmt::Display for SparsityRuleEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsityRuleEvidenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(
        id: &str,
        retained: u64,
        kind: ResourceEvidenceKind,
        protocol_id: &str,
        provenance_id: &str,
    ) -> ProvenancedSparsityRuleCandidate {
        ProvenancedSparsityRuleCandidate {
            candidate: SparsityRuleCandidate {
                candidate_id: id.to_owned(),
                phase: SparsityEvaluationPhase::Search,
                quality_loss_units: i64::try_from(retained).unwrap(),
                retained_units: retained,
                total_units: 100,
                controller_cost_units: 2,
                effective_compute_units: retained * 10,
                memory_traffic_bytes: retained * 64,
                latency_ns: retained * 100,
                rule_complexity_units: 2,
            },
            resource_evidence: ResourceEvidenceProvenance {
                kind,
                protocol_id: protocol_id.to_owned(),
                provenance_id: provenance_id.to_owned(),
            },
        }
    }

    #[test]
    fn accepts_one_frozen_resource_contract() {
        let mut candidates = vec![
            item(
                "dense-quality",
                80,
                ResourceEvidenceKind::ReferenceModel,
                "bl14-reference-v1",
                "sha256:reference-v1",
            ),
            item(
                "sparse",
                40,
                ResourceEvidenceKind::ReferenceModel,
                "bl14-reference-v1",
                "sha256:reference-v1",
            ),
        ];
        candidates[0].candidate.quality_loss_units = 0;

        assert_eq!(
            pareto_frontier_indices_with_resource_provenance(&candidates).unwrap(),
            vec![0, 1]
        );
    }

    #[test]
    fn rejects_holdout_only_screening() {
        let mut holdout = item(
            "holdout",
            50,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-a",
        );
        holdout.candidate.phase = SparsityEvaluationPhase::Holdout;

        assert_eq!(
            pareto_frontier_indices_with_resource_provenance(&[holdout]),
            Err(SparsityRuleEvidenceError::HoldoutScreeningForbidden {
                candidate_id: "holdout".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_modelled_and_measured_resource_pooling() {
        let modelled = item(
            "modelled",
            50,
            ResourceEvidenceKind::ReferenceModel,
            "bl14-reference-v1",
            "evidence-a",
        );
        let measured = item(
            "measured",
            50,
            ResourceEvidenceKind::Measured,
            "bl14-reference-v1",
            "evidence-a",
        );

        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[modelled, measured]),
            Err(SparsityRuleEvidenceError::MixedResourceEvidenceKind { .. })
        ));
    }

    #[test]
    fn rejects_protocol_and_provenance_mismatch() {
        let first = item(
            "first",
            50,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-a",
        );
        let protocol_mismatch = item(
            "protocol",
            50,
            ResourceEvidenceKind::Measured,
            "thor-v1",
            "run-a",
        );
        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[first.clone(), protocol_mismatch,]),
            Err(SparsityRuleEvidenceError::ResourceProtocolMismatch { .. })
        ));

        let provenance_mismatch = item(
            "provenance",
            50,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-b",
        );
        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[first, provenance_mismatch]),
            Err(SparsityRuleEvidenceError::ResourceProvenanceMismatch { .. })
        ));
    }

    #[test]
    fn rejects_empty_resource_provenance() {
        let empty_protocol = item(
            "empty-protocol",
            50,
            ResourceEvidenceKind::ReferenceModel,
            " ",
            "evidence-a",
        );
        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[empty_protocol]),
            Err(SparsityRuleEvidenceError::EmptyProtocolId { .. })
        ));

        let empty_provenance = item(
            "empty-provenance",
            50,
            ResourceEvidenceKind::ReferenceModel,
            "bl14-reference-v1",
            "",
        );
        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[empty_provenance]),
            Err(SparsityRuleEvidenceError::EmptyProvenanceId { .. })
        ));
    }

    #[test]
    fn preserves_underlying_search_holdout_gate() {
        let mut search = item(
            "search",
            50,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-a",
        );
        let mut holdout = search.clone();
        search.candidate.phase = SparsityEvaluationPhase::Search;
        holdout.candidate.candidate_id = "holdout".to_owned();
        holdout.candidate.phase = SparsityEvaluationPhase::Holdout;

        assert!(matches!(
            pareto_frontier_indices_with_resource_provenance(&[search, holdout]),
            Err(SparsityRuleEvidenceError::Candidate(
                SparsityRuleSearchError::MixedEvaluationPhase { .. }
            ))
        ));
    }
}
