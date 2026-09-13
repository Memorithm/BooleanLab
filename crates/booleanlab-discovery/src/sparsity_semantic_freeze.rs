//! BL-14.5 exact rule-semantic freeze boundary.
//!
//! Candidate identifiers alone are not semantic identities. This layer binds a
//! frozen SEARCH selection to the complete [`BooleanFunction`] truth table for
//! every selected rule, then requires the final HOLDOUT batch to present exactly
//! the same id-to-function mapping. Equality is exact; the non-cryptographic
//! proposal fingerprint is never treated as proof of rule identity.

use std::collections::BTreeMap;
use std::fmt;

use crate::BooleanFunction;
use crate::sparsity_freeze::{FrozenSparsitySelection, SparsityFreezeError};
use crate::sparsity_rule_search::SparsityRuleCandidate;

/// Exact experiment-owned binding between one screening id and one Boolean rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSparsityRuleBinding {
    pub candidate_id: String,
    pub function: BooleanFunction,
}

/// Frozen id-to-rule mapping that must survive unchanged into final HOLDOUT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenSparsityRuleSelection {
    selection: FrozenSparsitySelection,
    rules: BTreeMap<String, BooleanFunction>,
}

/// Fail-closed errors for exact BL-14.5 semantic identity binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparsitySemanticFreezeError {
    Selection(SparsityFreezeError),
    EmptyBindingId {
        index: usize,
    },
    DuplicateBinding {
        candidate_id: String,
    },
    UnfrozenBinding {
        candidate_id: String,
    },
    MissingFrozenBinding {
        candidate_id: String,
    },
    RuleSemanticMismatch {
        candidate_id: String,
    },
}

impl FrozenSparsityRuleSelection {
    /// Bind an already-frozen SEARCH frontier to exact Boolean truth tables.
    ///
    /// Binding order is irrelevant; identity is by candidate id and exact
    /// [`BooleanFunction`] equality. The supplied set must contain every frozen
    /// id exactly once and no other id.
    ///
    /// # Errors
    ///
    /// Rejects empty/duplicate ids, ids not present in the frozen SEARCH
    /// frontier, and missing frozen ids.
    pub fn bind_search_rules(
        selection: FrozenSparsitySelection,
        bindings: &[ExactSparsityRuleBinding],
    ) -> Result<Self, SparsitySemanticFreezeError> {
        let rules = validate_binding_set(&selection, bindings)?;
        Ok(Self { selection, rules })
    }

    /// Underlying SEARCH-frontier freeze used for objective/domain validation.
    #[must_use]
    pub const fn selection(&self) -> &FrozenSparsitySelection {
        &self.selection
    }

    /// Validate final HOLDOUT objectives and exact rule semantics together.
    ///
    /// This first applies the ordinary SEARCH->HOLDOUT identity/domain gate,
    /// then verifies that every candidate id is still bound to the exact same
    /// Boolean truth table frozen after SEARCH selection. No HOLDOUT objective
    /// is used to alter the frozen set or rule semantics.
    ///
    /// # Errors
    ///
    /// Propagates [`SparsityFreezeError`] for invalid HOLDOUT evidence and
    /// rejects missing/extra/duplicate bindings or any exact truth-table drift.
    pub fn validate_holdout(
        &self,
        candidates: &[SparsityRuleCandidate],
        bindings: &[ExactSparsityRuleBinding],
    ) -> Result<(), SparsitySemanticFreezeError> {
        self.selection
            .validate_holdout(candidates)
            .map_err(SparsitySemanticFreezeError::Selection)?;

        let holdout_rules = validate_binding_set(&self.selection, bindings)?;
        for (candidate_id, frozen_function) in &self.rules {
            let holdout_function = holdout_rules
                .get(candidate_id)
                .expect("validated binding set contains every frozen id");
            if holdout_function != frozen_function {
                return Err(SparsitySemanticFreezeError::RuleSemanticMismatch {
                    candidate_id: candidate_id.clone(),
                });
            }
        }
        Ok(())
    }
}

fn validate_binding_set(
    selection: &FrozenSparsitySelection,
    bindings: &[ExactSparsityRuleBinding],
) -> Result<BTreeMap<String, BooleanFunction>, SparsitySemanticFreezeError> {
    let mut rules = BTreeMap::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.candidate_id.is_empty() {
            return Err(SparsitySemanticFreezeError::EmptyBindingId { index });
        }
        if !selection
            .candidate_ids()
            .iter()
            .any(|candidate_id| candidate_id == &binding.candidate_id)
        {
            return Err(SparsitySemanticFreezeError::UnfrozenBinding {
                candidate_id: binding.candidate_id.clone(),
            });
        }
        if rules
            .insert(binding.candidate_id.clone(), binding.function.clone())
            .is_some()
        {
            return Err(SparsitySemanticFreezeError::DuplicateBinding {
                candidate_id: binding.candidate_id.clone(),
            });
        }
    }

    for candidate_id in selection.candidate_ids() {
        if !rules.contains_key(candidate_id) {
            return Err(SparsitySemanticFreezeError::MissingFrozenBinding {
                candidate_id: candidate_id.clone(),
            });
        }
    }
    Ok(rules)
}

impl fmt::Display for SparsitySemanticFreezeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsitySemanticFreezeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparsity_rule_search::{SparsityEvaluationPhase, SparsityRuleCandidate};

    fn candidate(candidate_id: &str, phase: SparsityEvaluationPhase, retained: u64) -> SparsityRuleCandidate {
        SparsityRuleCandidate {
            candidate_id: candidate_id.to_owned(),
            phase,
            quality_loss_units: 0,
            retained_units: retained,
            total_units: 100,
            controller_cost_units: 1,
            effective_compute_units: retained * 10,
            memory_traffic_bytes: retained * 64,
            latency_ns: retained * 100,
            rule_complexity_units: 1,
        }
    }

    fn binding(candidate_id: &str, truth_table: Vec<u8>) -> ExactSparsityRuleBinding {
        ExactSparsityRuleBinding {
            candidate_id: candidate_id.to_owned(),
            function: BooleanFunction::new(2, truth_table).unwrap(),
        }
    }

    fn frozen_rules() -> FrozenSparsityRuleSelection {
        let search = vec![
            candidate("a", SparsityEvaluationPhase::Search, 80),
            candidate("b", SparsityEvaluationPhase::Search, 40),
        ];
        let selection = FrozenSparsitySelection::from_search_frontier(&search).unwrap();
        FrozenSparsityRuleSelection::bind_search_rules(
            selection,
            &[
                binding("a", vec![0, 0, 0, 1]),
                binding("b", vec![0, 1, 1, 0]),
            ],
        )
        .unwrap()
    }

    #[test]
    fn accepts_exact_rules_even_when_holdout_binding_order_changes() {
        let frozen = frozen_rules();
        let holdout = vec![
            candidate("b", SparsityEvaluationPhase::Holdout, 39),
            candidate("a", SparsityEvaluationPhase::Holdout, 81),
        ];
        let bindings = vec![
            binding("b", vec![0, 1, 1, 0]),
            binding("a", vec![0, 0, 0, 1]),
        ];
        assert_eq!(frozen.validate_holdout(&holdout, &bindings), Ok(()));
    }

    #[test]
    fn rejects_same_id_rebound_to_different_truth_table() {
        let frozen = frozen_rules();
        let holdout = vec![
            candidate("a", SparsityEvaluationPhase::Holdout, 81),
            candidate("b", SparsityEvaluationPhase::Holdout, 39),
        ];
        let bindings = vec![
            binding("a", vec![0, 1, 1, 1]),
            binding("b", vec![0, 1, 1, 0]),
        ];
        assert_eq!(
            frozen.validate_holdout(&holdout, &bindings),
            Err(SparsitySemanticFreezeError::RuleSemanticMismatch {
                candidate_id: "a".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_missing_extra_and_duplicate_rule_bindings() {
        let search = vec![
            candidate("a", SparsityEvaluationPhase::Search, 80),
            candidate("b", SparsityEvaluationPhase::Search, 40),
        ];
        let selection = FrozenSparsitySelection::from_search_frontier(&search).unwrap();

        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection.clone(),
                &[binding("a", vec![0, 0, 0, 1])],
            ),
            Err(SparsitySemanticFreezeError::MissingFrozenBinding {
                candidate_id: "b".to_owned(),
            })
        );
        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection.clone(),
                &[
                    binding("a", vec![0, 0, 0, 1]),
                    binding("b", vec![0, 1, 1, 0]),
                    binding("extra", vec![0, 0, 1, 1]),
                ],
            ),
            Err(SparsitySemanticFreezeError::UnfrozenBinding {
                candidate_id: "extra".to_owned(),
            })
        );
        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection,
                &[
                    binding("a", vec![0, 0, 0, 1]),
                    binding("a", vec![0, 0, 0, 1]),
                    binding("b", vec![0, 1, 1, 0]),
                ],
            ),
            Err(SparsitySemanticFreezeError::DuplicateBinding {
                candidate_id: "a".to_owned(),
            })
        );
    }

    #[test]
    fn semantic_validation_still_requires_holdout_phase() {
        let frozen = frozen_rules();
        let search_again = vec![
            candidate("a", SparsityEvaluationPhase::Search, 80),
            candidate("b", SparsityEvaluationPhase::Search, 40),
        ];
        let bindings = vec![
            binding("a", vec![0, 0, 0, 1]),
            binding("b", vec![0, 1, 1, 0]),
        ];
        assert!(matches!(
            frozen.validate_holdout(&search_again, &bindings),
            Err(SparsitySemanticFreezeError::Selection(
                SparsityFreezeError::HoldoutPhaseRequired { .. }
            ))
        ));
    }
}
