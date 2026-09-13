//! BL-14.5 search-to-holdout freeze boundary.
//!
//! Search evidence may select a bounded set of rule identities. Once frozen,
//! final HOLDOUT evaluation must contain exactly those identities, on the same
//! declared evaluation-domain size, without adding, dropping, or duplicating a
//! candidate. The holdout outcomes are never used to revise the frozen set.

use std::collections::BTreeSet;
use std::fmt;

use crate::sparsity_rule_search::{
    SparsityEvaluationPhase, SparsityRuleCandidate, SparsityRuleSearchError,
    pareto_frontier_indices,
};

/// Immutable identity boundary between BL-14.5 SEARCH selection and HOLDOUT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenSparsitySelection {
    candidate_ids: Vec<String>,
    total_units: u64,
}

/// Fail-closed errors for the BL-14.5 SEARCH -> HOLDOUT boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparsityFreezeError {
    Search(SparsityRuleSearchError),
    EmptyFrozenSelection,
    SearchPhaseRequired {
        candidate_id: String,
        actual: SparsityEvaluationPhase,
    },
    HoldoutIsEmpty,
    HoldoutPhaseRequired {
        candidate_id: String,
        actual: SparsityEvaluationPhase,
    },
    HoldoutDomainSizeMismatch {
        candidate_id: String,
        expected_total_units: u64,
        actual_total_units: u64,
    },
    HoldoutRetainedUnitsExceedTotal {
        candidate_id: String,
        retained_units: u64,
        total_units: u64,
    },
    DuplicateHoldoutCandidate {
        candidate_id: String,
    },
    UnfrozenHoldoutCandidate {
        candidate_id: String,
    },
    MissingFrozenCandidate {
        candidate_id: String,
    },
}

impl FrozenSparsitySelection {
    /// Freeze exactly the non-dominated SEARCH candidates under the existing
    /// BL-14.5 Pareto contract.
    ///
    /// Candidate order is retained from the SEARCH population. The frozen set
    /// stores identities and the matched domain size only; no HOLDOUT outcome
    /// can change this selection.
    ///
    /// # Errors
    ///
    /// Rejects any non-SEARCH row before objective values can influence the
    /// frozen selection. Propagates [`SparsityRuleSearchError`] when SEARCH
    /// evidence is malformed. Returns [`SparsityFreezeError::EmptyFrozenSelection`]
    /// if the validated search population unexpectedly yields no frontier candidate.
    pub fn from_search_frontier(
        candidates: &[SparsityRuleCandidate],
    ) -> Result<Self, SparsityFreezeError> {
        for candidate in candidates {
            if candidate.phase != SparsityEvaluationPhase::Search {
                return Err(SparsityFreezeError::SearchPhaseRequired {
                    candidate_id: candidate.candidate_id.clone(),
                    actual: candidate.phase,
                });
            }
        }

        let frontier = pareto_frontier_indices(candidates).map_err(SparsityFreezeError::Search)?;
        let Some(first_index) = frontier.first().copied() else {
            return Err(SparsityFreezeError::EmptyFrozenSelection);
        };
        let total_units = candidates[first_index].total_units;
        let candidate_ids = frontier
            .into_iter()
            .map(|index| candidates[index].candidate_id.clone())
            .collect::<Vec<_>>();

        Ok(Self {
            candidate_ids,
            total_units,
        })
    }

    /// Frozen candidate ids in deterministic SEARCH-frontier order.
    #[must_use]
    pub fn candidate_ids(&self) -> &[String] {
        &self.candidate_ids
    }

    /// Matched evaluation-domain size frozen at SEARCH selection time.
    #[must_use]
    pub const fn total_units(&self) -> u64 {
        self.total_units
    }

    /// Validate a final HOLDOUT batch without reading its objective values for
    /// selection. The batch must contain exactly the frozen candidate identities
    /// once each and use the same declared domain size.
    ///
    /// HOLDOUT row order may differ because identity, not incidental row order,
    /// binds the final evaluation to the frozen SEARCH selection.
    ///
    /// # Errors
    ///
    /// Rejects an empty batch, non-HOLDOUT evidence, changed domain size,
    /// impossible retained cardinality, duplicate identities, newly introduced
    /// candidates, or missing frozen candidates.
    pub fn validate_holdout(
        &self,
        candidates: &[SparsityRuleCandidate],
    ) -> Result<(), SparsityFreezeError> {
        if candidates.is_empty() {
            return Err(SparsityFreezeError::HoldoutIsEmpty);
        }

        let frozen = self
            .candidate_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut observed = BTreeSet::new();

        for candidate in candidates {
            if candidate.phase != SparsityEvaluationPhase::Holdout {
                return Err(SparsityFreezeError::HoldoutPhaseRequired {
                    candidate_id: candidate.candidate_id.clone(),
                    actual: candidate.phase,
                });
            }
            if candidate.total_units != self.total_units {
                return Err(SparsityFreezeError::HoldoutDomainSizeMismatch {
                    candidate_id: candidate.candidate_id.clone(),
                    expected_total_units: self.total_units,
                    actual_total_units: candidate.total_units,
                });
            }
            if candidate.retained_units > candidate.total_units {
                return Err(SparsityFreezeError::HoldoutRetainedUnitsExceedTotal {
                    candidate_id: candidate.candidate_id.clone(),
                    retained_units: candidate.retained_units,
                    total_units: candidate.total_units,
                });
            }
            if !observed.insert(candidate.candidate_id.as_str()) {
                return Err(SparsityFreezeError::DuplicateHoldoutCandidate {
                    candidate_id: candidate.candidate_id.clone(),
                });
            }
            if !frozen.contains(candidate.candidate_id.as_str()) {
                return Err(SparsityFreezeError::UnfrozenHoldoutCandidate {
                    candidate_id: candidate.candidate_id.clone(),
                });
            }
        }

        for candidate_id in &self.candidate_ids {
            if !observed.contains(candidate_id.as_str()) {
                return Err(SparsityFreezeError::MissingFrozenCandidate {
                    candidate_id: candidate_id.clone(),
                });
            }
        }

        Ok(())
    }
}

impl fmt::Display for SparsityFreezeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsityFreezeError {}

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
    fn freezes_only_search_frontier_and_accepts_exact_holdout_identity_set() {
        let search = vec![
            candidate("quality", SparsityEvaluationPhase::Search, 0, 80, 5),
            candidate("sparse", SparsityEvaluationPhase::Search, 4, 40, 4),
            candidate("dominated", SparsityEvaluationPhase::Search, 6, 60, 8),
            candidate("compact", SparsityEvaluationPhase::Search, 2, 60, 1),
        ];
        let frozen = FrozenSparsitySelection::from_search_frontier(&search).unwrap();
        assert_eq!(frozen.candidate_ids(), &["quality", "sparse", "compact"]);
        assert_eq!(frozen.total_units(), 100);

        let holdout = vec![
            candidate("compact", SparsityEvaluationPhase::Holdout, 9, 62, 1),
            candidate("quality", SparsityEvaluationPhase::Holdout, 3, 81, 5),
            candidate("sparse", SparsityEvaluationPhase::Holdout, 12, 39, 4),
        ];
        assert_eq!(frozen.validate_holdout(&holdout), Ok(()));
    }

    #[test]
    fn freeze_rejects_holdout_evidence_before_frontier_selection() {
        let holdout = vec![candidate(
            "final-only",
            SparsityEvaluationPhase::Holdout,
            -10,
            1,
            1,
        )];
        assert_eq!(
            FrozenSparsitySelection::from_search_frontier(&holdout),
            Err(SparsityFreezeError::SearchPhaseRequired {
                candidate_id: "final-only".to_owned(),
                actual: SparsityEvaluationPhase::Holdout,
            })
        );
    }

    #[test]
    fn holdout_cannot_add_or_drop_search_selected_candidates() {
        let search = vec![
            candidate("a", SparsityEvaluationPhase::Search, 0, 80, 5),
            candidate("b", SparsityEvaluationPhase::Search, 4, 40, 4),
        ];
        let frozen = FrozenSparsitySelection::from_search_frontier(&search).unwrap();

        let with_extra = vec![
            candidate("a", SparsityEvaluationPhase::Holdout, 1, 80, 5),
            candidate("b", SparsityEvaluationPhase::Holdout, 5, 40, 4),
            candidate("new", SparsityEvaluationPhase::Holdout, 0, 20, 2),
        ];
        assert_eq!(
            frozen.validate_holdout(&with_extra),
            Err(SparsityFreezeError::UnfrozenHoldoutCandidate {
                candidate_id: "new".to_owned(),
            })
        );

        let missing = vec![candidate("a", SparsityEvaluationPhase::Holdout, 1, 80, 5)];
        assert_eq!(
            frozen.validate_holdout(&missing),
            Err(SparsityFreezeError::MissingFrozenCandidate {
                candidate_id: "b".to_owned(),
            })
        );
    }

    #[test]
    fn holdout_rejects_phase_domain_cardinality_and_duplicate_drift() {
        let search = vec![candidate("only", SparsityEvaluationPhase::Search, 0, 50, 1)];
        let frozen = FrozenSparsitySelection::from_search_frontier(&search).unwrap();

        let wrong_phase = vec![candidate("only", SparsityEvaluationPhase::Search, 0, 50, 1)];
        assert!(matches!(
            frozen.validate_holdout(&wrong_phase),
            Err(SparsityFreezeError::HoldoutPhaseRequired { .. })
        ));

        let mut wrong_domain = candidate("only", SparsityEvaluationPhase::Holdout, 0, 50, 1);
        wrong_domain.total_units = 101;
        assert!(matches!(
            frozen.validate_holdout(&[wrong_domain]),
            Err(SparsityFreezeError::HoldoutDomainSizeMismatch { .. })
        ));

        let impossible = candidate("only", SparsityEvaluationPhase::Holdout, 0, 101, 1);
        assert_eq!(
            frozen.validate_holdout(&[impossible]),
            Err(SparsityFreezeError::HoldoutRetainedUnitsExceedTotal {
                candidate_id: "only".to_owned(),
                retained_units: 101,
                total_units: 100,
            })
        );

        let duplicate = candidate("only", SparsityEvaluationPhase::Holdout, 0, 50, 1);
        assert!(matches!(
            frozen.validate_holdout(&[duplicate.clone(), duplicate]),
            Err(SparsityFreezeError::DuplicateHoldoutCandidate { .. })
        ));
    }
}
