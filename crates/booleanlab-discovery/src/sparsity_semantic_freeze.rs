//! BL-14.5 exact rule-semantic freeze boundary.
//!
//! Candidate identifiers alone are not semantic identities. This layer binds a
//! frozen SEARCH selection to the complete [`BooleanFunction`] truth table, the
//! canonical predicate-schema/configuration definition, and the resolved
//! predicate parameters used to construct its Boolean inputs. Final HOLDOUT
//! must present exactly the same mapping. Equality is exact; the
//! non-cryptographic proposal fingerprint is never treated as proof of rule
//! identity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::BooleanFunction;
use crate::sparsity_freeze::{FrozenSparsitySelection, SparsityFreezeError};
use crate::sparsity_rule_search::SparsityRuleCandidate;

/// One resolved predicate parameter frozen after SEARCH.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedPredicateParameter {
    pub predicate_id: String,
    pub parameter_name: String,
    pub resolved_value: String,
}

/// Exact experiment-owned binding between one screening id and one Boolean rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSparsityRuleBinding {
    pub candidate_id: String,
    pub function: BooleanFunction,
    pub predicate_schema: String,
    pub resolved_parameters: Vec<ResolvedPredicateParameter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrozenRuleSemantics {
    function: BooleanFunction,
    predicate_schema: String,
    resolved_parameters: Vec<ResolvedPredicateParameter>,
}

/// Frozen id-to-rule mapping that must survive unchanged into final HOLDOUT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenSparsityRuleSelection {
    selection: FrozenSparsitySelection,
    rules: BTreeMap<String, FrozenRuleSemantics>,
}

/// Fail-closed errors for exact BL-14.5 semantic identity binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparsitySemanticFreezeError {
    Selection(SparsityFreezeError),
    EmptyBindingId { index: usize },
    EmptyPredicateSchema { candidate_id: String },
    EmptyResolvedParameterField {
        candidate_id: String,
        parameter_index: usize,
    },
    DuplicateResolvedParameter {
        candidate_id: String,
        predicate_id: String,
        parameter_name: String,
    },
    DuplicateBinding { candidate_id: String },
    UnfrozenBinding { candidate_id: String },
    MissingFrozenBinding { candidate_id: String },
    RuleSemanticMismatch { candidate_id: String },
    PredicateSchemaMismatch { candidate_id: String },
    ResolvedPredicateParametersMismatch { candidate_id: String },
}

impl FrozenSparsityRuleSelection {
    /// Bind an already-frozen SEARCH frontier to exact Boolean truth tables,
    /// canonical predicate schemas, and concrete SEARCH-resolved parameters.
    ///
    /// # Errors
    ///
    /// Rejects empty/duplicate ids, empty predicate schemas, malformed or
    /// duplicate resolved parameters, ids not present in the frozen SEARCH
    /// frontier, and missing frozen ids.
    pub fn bind_search_rules(
        selection: FrozenSparsitySelection,
        bindings: &[ExactSparsityRuleBinding],
    ) -> Result<Self, SparsitySemanticFreezeError> {
        let rules = validate_binding_set(&selection, bindings)?;
        Ok(Self { selection, rules })
    }

    #[must_use]
    pub const fn selection(&self) -> &FrozenSparsitySelection {
        &self.selection
    }

    /// Validate final HOLDOUT objectives and exact rule semantics together.
    ///
    /// # Errors
    ///
    /// Propagates [`SparsityFreezeError`] for invalid HOLDOUT evidence and
    /// rejects missing/extra/duplicate bindings, truth-table drift, predicate
    /// schema drift, resolved-parameter drift, or malformed predicate state.
    pub fn validate_holdout(
        &self,
        candidates: &[SparsityRuleCandidate],
        bindings: &[ExactSparsityRuleBinding],
    ) -> Result<(), SparsitySemanticFreezeError> {
        self.selection
            .validate_holdout(candidates)
            .map_err(SparsitySemanticFreezeError::Selection)?;

        let holdout_rules = validate_binding_set(&self.selection, bindings)?;
        for (candidate_id, frozen_semantics) in &self.rules {
            let holdout_semantics = holdout_rules.get(candidate_id).ok_or_else(|| {
                SparsitySemanticFreezeError::MissingFrozenBinding {
                    candidate_id: candidate_id.clone(),
                }
            })?;
            if holdout_semantics.function != frozen_semantics.function {
                return Err(SparsitySemanticFreezeError::RuleSemanticMismatch {
                    candidate_id: candidate_id.clone(),
                });
            }
            if holdout_semantics.predicate_schema != frozen_semantics.predicate_schema {
                return Err(SparsitySemanticFreezeError::PredicateSchemaMismatch {
                    candidate_id: candidate_id.clone(),
                });
            }
            if holdout_semantics.resolved_parameters != frozen_semantics.resolved_parameters {
                return Err(
                    SparsitySemanticFreezeError::ResolvedPredicateParametersMismatch {
                        candidate_id: candidate_id.clone(),
                    },
                );
            }
        }
        Ok(())
    }
}

fn validate_binding_set(
    selection: &FrozenSparsitySelection,
    bindings: &[ExactSparsityRuleBinding],
) -> Result<BTreeMap<String, FrozenRuleSemantics>, SparsitySemanticFreezeError> {
    let mut rules = BTreeMap::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.candidate_id.is_empty() {
            return Err(SparsitySemanticFreezeError::EmptyBindingId { index });
        }
        if binding.predicate_schema.is_empty() {
            return Err(SparsitySemanticFreezeError::EmptyPredicateSchema {
                candidate_id: binding.candidate_id.clone(),
            });
        }
        validate_resolved_parameters(binding)?;
        if !selection
            .candidate_ids()
            .iter()
            .any(|candidate_id| candidate_id == &binding.candidate_id)
        {
            return Err(SparsitySemanticFreezeError::UnfrozenBinding {
                candidate_id: binding.candidate_id.clone(),
            });
        }
        let semantics = FrozenRuleSemantics {
            function: binding.function.clone(),
            predicate_schema: binding.predicate_schema.clone(),
            resolved_parameters: binding.resolved_parameters.clone(),
        };
        if rules
            .insert(binding.candidate_id.clone(), semantics)
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

fn validate_resolved_parameters(
    binding: &ExactSparsityRuleBinding,
) -> Result<(), SparsitySemanticFreezeError> {
    let mut keys = BTreeSet::new();
    for (parameter_index, parameter) in binding.resolved_parameters.iter().enumerate() {
        if parameter.predicate_id.is_empty()
            || parameter.parameter_name.is_empty()
            || parameter.resolved_value.is_empty()
        {
            return Err(SparsitySemanticFreezeError::EmptyResolvedParameterField {
                candidate_id: binding.candidate_id.clone(),
                parameter_index,
            });
        }
        let key = (
            parameter.predicate_id.clone(),
            parameter.parameter_name.clone(),
        );
        if !keys.insert(key.clone()) {
            return Err(SparsitySemanticFreezeError::DuplicateResolvedParameter {
                candidate_id: binding.candidate_id.clone(),
                predicate_id: key.0,
                parameter_name: key.1,
            });
        }
    }
    Ok(())
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
            quality_loss_units: if retained >= 80 { 0 } else { 4 },
            retained_units: retained,
            total_units: 100,
            controller_cost_units: 1,
            effective_compute_units: retained * 10,
            memory_traffic_bytes: retained * 64,
            latency_ns: retained * 100,
            rule_complexity_units: 1,
        }
    }

    fn resolved_parameters(q75: &str) -> Vec<ResolvedPredicateParameter> {
        vec![
            ResolvedPredicateParameter {
                predicate_id: "magnitude".to_owned(),
                parameter_name: "threshold".to_owned(),
                resolved_value: q75.to_owned(),
            },
            ResolvedPredicateParameter {
                predicate_id: "activity".to_owned(),
                parameter_name: "window".to_owned(),
                resolved_value: "32".to_owned(),
            },
        ]
    }

    fn binding_with_semantics(
        candidate_id: &str,
        truth_table: Vec<u8>,
        predicate_schema: &str,
        parameters: Vec<ResolvedPredicateParameter>,
    ) -> ExactSparsityRuleBinding {
        ExactSparsityRuleBinding {
            candidate_id: candidate_id.to_owned(),
            function: BooleanFunction::new(2, truth_table).unwrap(),
            predicate_schema: predicate_schema.to_owned(),
            resolved_parameters: parameters,
        }
    }

    fn binding(candidate_id: &str, truth_table: Vec<u8>) -> ExactSparsityRuleBinding {
        binding_with_semantics(
            candidate_id,
            truth_table,
            "v1:[magnitude>=q75,activity_window=32]",
            resolved_parameters("0.750000"),
        )
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
    fn rejects_same_truth_table_with_changed_predicate_schema() {
        let frozen = frozen_rules();
        let holdout = vec![
            candidate("a", SparsityEvaluationPhase::Holdout, 81),
            candidate("b", SparsityEvaluationPhase::Holdout, 39),
        ];
        let bindings = vec![
            binding_with_semantics(
                "a",
                vec![0, 0, 0, 1],
                "v1:[activity_window=32,magnitude>=q75]",
                resolved_parameters("0.750000"),
            ),
            binding("b", vec![0, 1, 1, 0]),
        ];
        assert_eq!(
            frozen.validate_holdout(&holdout, &bindings),
            Err(SparsitySemanticFreezeError::PredicateSchemaMismatch {
                candidate_id: "a".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_same_schema_with_refit_resolved_threshold() {
        let frozen = frozen_rules();
        let holdout = vec![
            candidate("a", SparsityEvaluationPhase::Holdout, 81),
            candidate("b", SparsityEvaluationPhase::Holdout, 39),
        ];
        let bindings = vec![
            binding_with_semantics(
                "a",
                vec![0, 0, 0, 1],
                "v1:[magnitude>=q75,activity_window=32]",
                resolved_parameters("0.812500"),
            ),
            binding("b", vec![0, 1, 1, 0]),
        ];
        assert_eq!(
            frozen.validate_holdout(&holdout, &bindings),
            Err(SparsitySemanticFreezeError::ResolvedPredicateParametersMismatch {
                candidate_id: "a".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_empty_predicate_schema() {
        let search = vec![candidate("a", SparsityEvaluationPhase::Search, 80)];
        let selection = FrozenSparsitySelection::from_search_frontier(&search).unwrap();
        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection,
                &[binding_with_semantics(
                    "a",
                    vec![0, 0, 0, 1],
                    "",
                    resolved_parameters("0.750000"),
                )],
            ),
            Err(SparsitySemanticFreezeError::EmptyPredicateSchema {
                candidate_id: "a".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_resolved_parameters() {
        let search = vec![candidate("a", SparsityEvaluationPhase::Search, 80)];
        let selection = FrozenSparsitySelection::from_search_frontier(&search).unwrap();
        let malformed = vec![ResolvedPredicateParameter {
            predicate_id: "magnitude".to_owned(),
            parameter_name: "threshold".to_owned(),
            resolved_value: String::new(),
        }];
        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection.clone(),
                &[binding_with_semantics(
                    "a",
                    vec![0, 0, 0, 1],
                    "v1:[magnitude>=q75]",
                    malformed,
                )],
            ),
            Err(SparsitySemanticFreezeError::EmptyResolvedParameterField {
                candidate_id: "a".to_owned(),
                parameter_index: 0,
            })
        );

        let duplicate = ResolvedPredicateParameter {
            predicate_id: "magnitude".to_owned(),
            parameter_name: "threshold".to_owned(),
            resolved_value: "0.750000".to_owned(),
        };
        assert_eq!(
            FrozenSparsityRuleSelection::bind_search_rules(
                selection,
                &[binding_with_semantics(
                    "a",
                    vec![0, 0, 0, 1],
                    "v1:[magnitude>=q75]",
                    vec![duplicate.clone(), duplicate],
                )],
            ),
            Err(SparsitySemanticFreezeError::DuplicateResolvedParameter {
                candidate_id: "a".to_owned(),
                predicate_id: "magnitude".to_owned(),
                parameter_name: "threshold".to_owned(),
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
