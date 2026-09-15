//! BL-14.5 post-freeze HOLDOUT reporting without reselection.
//!
//! SEARCH may choose a bounded set of rule identities and freeze their exact
//! Boolean semantics. Final `HOLDOUT` observations may then be reported, but
//! they must never be ranked, Pareto-screened, or used to revise that frozen
//! selection. This module validates the complete freeze boundary plus one
//! resource-evidence contract, then emits rows in the original SEARCH order.

use std::collections::BTreeMap;
use std::fmt;

use crate::sparsity_rule_evidence::{
    ProvenancedSparsityRuleCandidate, ResourceEvidenceKind, ResourceEvidenceProvenance,
};
use crate::sparsity_rule_search::SparsityRuleCandidate;
use crate::sparsity_semantic_freeze::{
    ExactSparsityRuleBinding, FrozenSparsityRuleSelection, SparsitySemanticFreezeError,
};

/// Final resource report for exactly one frozen `HOLDOUT` evaluation contract.
///
/// `rows` are ordered by the immutable SEARCH selection, never by any `HOLDOUT`
/// objective. The report deliberately exposes no frontier, winner, or ranking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldoutResourceReport {
    pub resource_evidence: ResourceEvidenceProvenance,
    pub rows: Vec<ProvenancedSparsityRuleCandidate>,
}

/// Fail-closed errors for post-freeze `HOLDOUT` reporting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoldoutReportError {
    SemanticFreeze(SparsitySemanticFreezeError),
    EmptyProtocolId {
        candidate_id: String,
    },
    EmptyProvenanceId {
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
    ValidatedCandidateMissing {
        candidate_id: String,
    },
}

/// Validate and report final `HOLDOUT` observations for one frozen SEARCH result.
///
/// This function first verifies candidate identity, evaluation phase, matched
/// domain size, exact Boolean truth tables, predicate schema, and resolved
/// predicate parameters through [`FrozenSparsityRuleSelection::validate_holdout`].
/// It then requires every resource row to use one non-empty evidence contract.
/// Finally, it returns rows in frozen SEARCH order. No `HOLDOUT` objective is
/// read for selection, sorting, dominance, or tie-breaking.
///
/// # Errors
///
/// Returns [`HoldoutReportError`] for any freeze/semantic drift, empty or mixed
/// resource provenance, or a violated post-validation identity invariant.
pub fn build_holdout_resource_report(
    frozen: &FrozenSparsityRuleSelection,
    candidates: &[ProvenancedSparsityRuleCandidate],
    bindings: &[ExactSparsityRuleBinding],
) -> Result<HoldoutResourceReport, HoldoutReportError> {
    let bare = candidates
        .iter()
        .map(|item| item.candidate.clone())
        .collect::<Vec<SparsityRuleCandidate>>();
    frozen
        .validate_holdout(&bare, bindings)
        .map_err(HoldoutReportError::SemanticFreeze)?;

    let first = &candidates[0];
    validate_non_empty_provenance(first)?;
    let expected = &first.resource_evidence;

    for item in candidates.iter().skip(1) {
        validate_non_empty_provenance(item)?;
        let actual = &item.resource_evidence;
        if actual.kind != expected.kind {
            return Err(HoldoutReportError::MixedResourceEvidenceKind {
                expected: expected.kind,
                actual: actual.kind,
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
        if actual.protocol_id != expected.protocol_id {
            return Err(HoldoutReportError::ResourceProtocolMismatch {
                expected: expected.protocol_id.clone(),
                actual: actual.protocol_id.clone(),
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
        if actual.provenance_id != expected.provenance_id {
            return Err(HoldoutReportError::ResourceProvenanceMismatch {
                expected: expected.provenance_id.clone(),
                actual: actual.provenance_id.clone(),
                candidate_id: item.candidate.candidate_id.clone(),
            });
        }
    }

    let mut by_id = candidates
        .iter()
        .cloned()
        .map(|item| (item.candidate.candidate_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::with_capacity(frozen.selection().candidate_ids().len());
    for candidate_id in frozen.selection().candidate_ids() {
        let item = by_id.remove(candidate_id).ok_or_else(|| {
            HoldoutReportError::ValidatedCandidateMissing {
                candidate_id: candidate_id.clone(),
            }
        })?;
        rows.push(item);
    }

    Ok(HoldoutResourceReport {
        resource_evidence: expected.clone(),
        rows,
    })
}

fn validate_non_empty_provenance(
    item: &ProvenancedSparsityRuleCandidate,
) -> Result<(), HoldoutReportError> {
    if item.resource_evidence.protocol_id.trim().is_empty() {
        return Err(HoldoutReportError::EmptyProtocolId {
            candidate_id: item.candidate.candidate_id.clone(),
        });
    }
    if item.resource_evidence.provenance_id.trim().is_empty() {
        return Err(HoldoutReportError::EmptyProvenanceId {
            candidate_id: item.candidate.candidate_id.clone(),
        });
    }
    Ok(())
}

impl fmt::Display for HoldoutReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for HoldoutReportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BooleanFunction;
    use crate::sparsity_freeze::FrozenSparsitySelection;
    use crate::sparsity_rule_search::{SparsityEvaluationPhase, SparsityRuleCandidate};

    fn candidate(
        candidate_id: &str,
        phase: SparsityEvaluationPhase,
        quality_loss_units: i64,
        retained_units: u64,
    ) -> SparsityRuleCandidate {
        SparsityRuleCandidate {
            candidate_id: candidate_id.to_owned(),
            phase,
            quality_loss_units,
            retained_units,
            total_units: 100,
            controller_cost_units: if candidate_id == "a" { 5 } else { 4 },
            effective_compute_units: retained_units * 10,
            memory_traffic_bytes: retained_units * 64,
            latency_ns: retained_units * 100,
            rule_complexity_units: 2,
        }
    }

    fn binding(candidate_id: &str) -> ExactSparsityRuleBinding {
        let truth_table = if candidate_id == "a" {
            vec![0, 1]
        } else {
            vec![1, 0]
        };
        ExactSparsityRuleBinding {
            candidate_id: candidate_id.to_owned(),
            function: BooleanFunction::new(1, truth_table).unwrap(),
            predicate_schema: "v1:[activity>=threshold]".to_owned(),
            resolved_parameters: Vec::new(),
        }
    }

    fn frozen_rules() -> FrozenSparsityRuleSelection {
        let search = vec![
            candidate("a", SparsityEvaluationPhase::Search, 0, 80),
            candidate("b", SparsityEvaluationPhase::Search, 4, 40),
        ];
        let selection = FrozenSparsitySelection::from_search_frontier(&search).unwrap();
        assert_eq!(selection.candidate_ids(), &["a", "b"]);
        FrozenSparsityRuleSelection::bind_search_rules(
            selection,
            &[binding("a"), binding("b")],
        )
        .unwrap()
    }

    fn holdout_item(
        candidate_id: &str,
        quality_loss_units: i64,
        retained_units: u64,
        kind: ResourceEvidenceKind,
        protocol_id: &str,
        provenance_id: &str,
    ) -> ProvenancedSparsityRuleCandidate {
        ProvenancedSparsityRuleCandidate {
            candidate: candidate(
                candidate_id,
                SparsityEvaluationPhase::Holdout,
                quality_loss_units,
                retained_units,
            ),
            resource_evidence: ResourceEvidenceProvenance {
                kind,
                protocol_id: protocol_id.to_owned(),
                provenance_id: provenance_id.to_owned(),
            },
        }
    }

    #[test]
    fn reports_reordered_holdout_in_frozen_search_order() {
        let frozen = frozen_rules();
        let holdout = vec![
            holdout_item(
                "b",
                -20,
                10,
                ResourceEvidenceKind::Measured,
                "t430-v1",
                "run-001",
            ),
            holdout_item(
                "a",
                100,
                90,
                ResourceEvidenceKind::Measured,
                "t430-v1",
                "run-001",
            ),
        ];
        let report = build_holdout_resource_report(
            &frozen,
            &holdout,
            &[binding("b"), binding("a")],
        )
        .unwrap();

        assert_eq!(
            report
                .rows
                .iter()
                .map(|row| row.candidate.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn holdout_objective_changes_cannot_reorder_frozen_rows() {
        let frozen = frozen_rules();
        let first = vec![
            holdout_item(
                "a",
                100,
                90,
                ResourceEvidenceKind::ReferenceModel,
                "reference-v1",
                "evidence-001",
            ),
            holdout_item(
                "b",
                -100,
                10,
                ResourceEvidenceKind::ReferenceModel,
                "reference-v1",
                "evidence-001",
            ),
        ];
        let second = vec![
            holdout_item(
                "b",
                500,
                99,
                ResourceEvidenceKind::ReferenceModel,
                "reference-v1",
                "evidence-001",
            ),
            holdout_item(
                "a",
                -500,
                1,
                ResourceEvidenceKind::ReferenceModel,
                "reference-v1",
                "evidence-001",
            ),
        ];
        let bindings = [binding("a"), binding("b")];

        let first_report = build_holdout_resource_report(&frozen, &first, &bindings).unwrap();
        let second_report = build_holdout_resource_report(&frozen, &second, &bindings).unwrap();
        let ids = |report: &HoldoutResourceReport| {
            report
                .rows
                .iter()
                .map(|row| row.candidate.candidate_id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&first_report), vec!["a", "b"]);
        assert_eq!(ids(&second_report), vec!["a", "b"]);
    }

    #[test]
    fn rejects_search_observations_on_holdout_reporting_surface() {
        let frozen = frozen_rules();
        let mut search = holdout_item(
            "a",
            0,
            80,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-001",
        );
        search.candidate.phase = SparsityEvaluationPhase::Search;
        let other = holdout_item(
            "b",
            4,
            40,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-001",
        );

        assert!(matches!(
            build_holdout_resource_report(
                &frozen,
                &[search, other],
                &[binding("a"), binding("b")],
            ),
            Err(HoldoutReportError::SemanticFreeze(_))
        ));
    }

    #[test]
    fn rejects_mixed_resource_contracts() {
        let frozen = frozen_rules();
        let measured = holdout_item(
            "a",
            0,
            80,
            ResourceEvidenceKind::Measured,
            "t430-v1",
            "run-001",
        );
        let modelled = holdout_item(
            "b",
            4,
            40,
            ResourceEvidenceKind::ReferenceModel,
            "t430-v1",
            "run-001",
        );
        assert!(matches!(
            build_holdout_resource_report(
                &frozen,
                &[measured, modelled],
                &[binding("a"), binding("b")],
            ),
            Err(HoldoutReportError::MixedResourceEvidenceKind { .. })
        ));
    }

    #[test]
    fn rejects_semantic_drift_after_search_freeze() {
        let frozen = frozen_rules();
        let holdout = vec![
            holdout_item(
                "a",
                0,
                80,
                ResourceEvidenceKind::Measured,
                "t430-v1",
                "run-001",
            ),
            holdout_item(
                "b",
                4,
                40,
                ResourceEvidenceKind::Measured,
                "t430-v1",
                "run-001",
            ),
        ];
        let mut drifted = binding("b");
        drifted.function = BooleanFunction::new(1, vec![0, 1]).unwrap();

        assert!(matches!(
            build_holdout_resource_report(&frozen, &holdout, &[binding("a"), drifted]),
            Err(HoldoutReportError::SemanticFreeze(
                SparsitySemanticFreezeError::RuleSemanticMismatch { .. }
            ))
        ));
    }
}
