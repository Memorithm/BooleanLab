//! Bounded pairwise subsumption graphs for designated Strong-Kleene truth sets.
//!
//! This module lifts [`crate::kleene_designated_relation`] from one pair to a
//! bounded collection. It records only *strict* designated-`True` set
//! containment. Equal designated sets and incomparable pairs produce no edge.
//!
//! The graph is a BL-BE3 analysis artifact. It does not prove that a production
//! rule is removable: non-designated `False` versus `Unknown` semantics, rule
//! composition, provenance and runtime authorization remain outside this
//! contract. The pair limit bounds quadratic relation work but is not a timing
//! or memory-performance claim.

use crate::{
    KleeneDesignatedRelation, KleeneEntailmentError, KleeneSemanticKey, KleeneValue,
    kleene_designated_relation,
};

/// Default maximum number of unordered key pairs compared by the graph builder.
pub const DEFAULT_MAX_DESIGNATED_SUBSUMPTION_PAIRS: usize = 4_096;

/// Canonical witness that one strict designated-`True` set is smaller than another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneStrictSubsumptionWitness {
    /// Canonical base-3 assignment index.
    pub assignment_index: usize,
    /// Output of the more restrictive key on the witness row. This is not `True`.
    pub more_restrictive_output: KleeneValue,
    /// Output of the less restrictive key on the witness row. This is `True`.
    pub less_restrictive_output: KleeneValue,
}

/// One strict designated-`True` containment edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneDesignatedSubsumptionEdge {
    /// Index of the key whose designated-`True` rows form the strict subset.
    pub more_restrictive_index: usize,
    /// Index of the key whose designated-`True` rows form the strict superset.
    pub less_restrictive_index: usize,
    /// First canonical row proving strictness for this pair.
    pub witness: KleeneStrictSubsumptionWitness,
}

/// Bounded pairwise strict-subsumption graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KleeneDesignatedSubsumptionGraph {
    /// Number of input keys.
    pub node_count: usize,
    /// Number of unordered pairs examined.
    pub pair_count: usize,
    /// Strict containment edges in lexicographic pair order.
    pub edges: Vec<KleeneDesignatedSubsumptionEdge>,
}

/// Failure while building a bounded designated-value subsumption graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneDesignatedSubsumptionError {
    /// `n * (n - 1) / 2` could not be represented as `usize`.
    PairCountOverflow { node_count: usize },
    /// The declared pair budget would be exceeded.
    PairLimitExceeded {
        required_pairs: usize,
        max_pairs: usize,
    },
    /// Reserving storage for the worst-case edge count failed.
    AllocationFailed { requested_edges: usize },
    /// One exact pair relation failed structural/domain validation.
    Relation {
        left_index: usize,
        right_index: usize,
        error: KleeneEntailmentError,
    },
}

/// Builds the strict designated-`True` subsumption graph for exact semantic keys.
///
/// Every unordered pair is classified exactly with
/// [`kleene_designated_relation`]. Strict subset relations produce one directed
/// edge from the more restrictive key to the less restrictive key. Equal
/// designated sets and incomparable pairs produce no edge.
///
/// The pair count is checked before relation evaluation or edge allocation, so
/// callers can cap the quadratic part of the analysis deterministically. This
/// bound does not replace the row/work bounds used when constructing each
/// [`KleeneSemanticKey`].
///
/// # Errors
///
/// Returns [`KleeneDesignatedSubsumptionError::PairLimitExceeded`] before pair
/// evaluation when the declared budget is too small. Structural key errors and
/// domain mismatches preserve the exact pair indices that exposed them.
pub fn kleene_designated_subsumption_graph(
    keys: &[KleeneSemanticKey],
    max_pairs: usize,
) -> Result<KleeneDesignatedSubsumptionGraph, KleeneDesignatedSubsumptionError> {
    let pair_count = unordered_pair_count(keys.len())?;
    if pair_count > max_pairs {
        return Err(KleeneDesignatedSubsumptionError::PairLimitExceeded {
            required_pairs: pair_count,
            max_pairs,
        });
    }

    let mut edges = Vec::new();
    edges.try_reserve(pair_count).map_err(|_| {
        KleeneDesignatedSubsumptionError::AllocationFailed {
            requested_edges: pair_count,
        }
    })?;

    for left_index in 0..keys.len() {
        for right_index in (left_index + 1)..keys.len() {
            let relation = kleene_designated_relation(&keys[left_index], &keys[right_index])
                .map_err(|error| KleeneDesignatedSubsumptionError::Relation {
                    left_index,
                    right_index,
                    error,
                })?;

            match relation {
                KleeneDesignatedRelation::AntecedentMoreRestrictive {
                    consequent_only_witness,
                    ..
                } => edges.push(KleeneDesignatedSubsumptionEdge {
                    more_restrictive_index: left_index,
                    less_restrictive_index: right_index,
                    witness: KleeneStrictSubsumptionWitness {
                        assignment_index: consequent_only_witness.assignment_index,
                        more_restrictive_output: consequent_only_witness.antecedent,
                        less_restrictive_output: consequent_only_witness.consequent,
                    },
                }),
                KleeneDesignatedRelation::ConsequentMoreRestrictive {
                    antecedent_only_witness,
                    ..
                } => edges.push(KleeneDesignatedSubsumptionEdge {
                    more_restrictive_index: right_index,
                    less_restrictive_index: left_index,
                    witness: KleeneStrictSubsumptionWitness {
                        assignment_index: antecedent_only_witness.assignment_index,
                        more_restrictive_output: antecedent_only_witness.consequent,
                        less_restrictive_output: antecedent_only_witness.antecedent,
                    },
                }),
                KleeneDesignatedRelation::SameDesignatedTrueSet { .. }
                | KleeneDesignatedRelation::Incomparable { .. } => {}
            }
        }
    }

    Ok(KleeneDesignatedSubsumptionGraph {
        node_count: keys.len(),
        pair_count,
        edges,
    })
}

fn unordered_pair_count(node_count: usize) -> Result<usize, KleeneDesignatedSubsumptionError> {
    if node_count < 2 {
        return Ok(0);
    }

    node_count
        .checked_mul(node_count - 1)
        .and_then(|product| product.checked_div(2))
        .ok_or(KleeneDesignatedSubsumptionError::PairCountOverflow { node_count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KleeneInstruction, kleene_semantic_key};

    fn key(program: &[KleeneInstruction], input_arity: usize) -> KleeneSemanticKey {
        let mut rows = 1usize;
        for _ in 0..input_arity {
            rows = rows.checked_mul(3).expect("small test domain must fit");
        }
        let work = rows
            .checked_mul(program.len())
            .expect("small test work must fit");
        kleene_semantic_key(program, input_arity, rows, work)
            .expect("test program must produce an exact key")
    }

    #[test]
    fn empty_and_singleton_collections_have_no_pair_work() {
        let singleton = key(&[KleeneInstruction::Input(0)], 1);

        assert_eq!(
            kleene_designated_subsumption_graph(&[], 0),
            Ok(KleeneDesignatedSubsumptionGraph {
                node_count: 0,
                pair_count: 0,
                edges: Vec::new(),
            })
        );
        assert_eq!(
            kleene_designated_subsumption_graph(&[singleton], 0),
            Ok(KleeneDesignatedSubsumptionGraph {
                node_count: 1,
                pair_count: 0,
                edges: Vec::new(),
            })
        );
    }

    #[test]
    fn strict_subset_edges_are_oriented_from_more_to_less_restrictive() {
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let x_and_y = key(
            &[
                KleeneInstruction::Input(0),
                KleeneInstruction::Input(1),
                KleeneInstruction::And,
            ],
            2,
        );

        let graph =
            kleene_designated_subsumption_graph(&[x, x_and_y], 1).expect("one valid pair must fit");
        assert_eq!(graph.node_count, 2);
        assert_eq!(graph.pair_count, 1);
        assert_eq!(graph.edges.len(), 1);
        let edge = graph.edges[0];
        assert_eq!(edge.more_restrictive_index, 1);
        assert_eq!(edge.less_restrictive_index, 0);
        assert_ne!(edge.witness.more_restrictive_output, KleeneValue::True);
        assert_eq!(edge.witness.less_restrictive_output, KleeneValue::True);
    }

    #[test]
    fn equal_designated_sets_and_incomparable_pairs_create_no_edges() {
        let always_false = key(&[KleeneInstruction::Constant(KleeneValue::False)], 2);
        let always_unknown = key(&[KleeneInstruction::Constant(KleeneValue::Unknown)], 2);
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let y = key(&[KleeneInstruction::Input(1)], 2);

        let equal = kleene_designated_subsumption_graph(&[always_false, always_unknown], 1)
            .expect("equal designated sets are valid");
        assert!(equal.edges.is_empty());

        let incomparable =
            kleene_designated_subsumption_graph(&[x, y], 1).expect("incomparable keys are valid");
        assert!(incomparable.edges.is_empty());
    }

    #[test]
    fn pair_budget_is_checked_before_relation_evaluation() {
        let malformed = KleeneSemanticKey {
            schema_version: 1,
            input_arity: 1,
            rows: 3,
            packed_outputs: Vec::new(),
        };

        assert_eq!(
            kleene_designated_subsumption_graph(
                &[
                    malformed.clone(),
                    malformed,
                    key(&[KleeneInstruction::Input(0)], 1)
                ],
                2
            ),
            Err(KleeneDesignatedSubsumptionError::PairLimitExceeded {
                required_pairs: 3,
                max_pairs: 2,
            })
        );
    }

    #[test]
    fn relation_errors_preserve_pair_indices() {
        let arity_zero = key(&[KleeneInstruction::Constant(KleeneValue::True)], 0);
        let arity_one = key(&[KleeneInstruction::Constant(KleeneValue::True)], 1);

        assert_eq!(
            kleene_designated_subsumption_graph(&[arity_zero, arity_one], 1),
            Err(KleeneDesignatedSubsumptionError::Relation {
                left_index: 0,
                right_index: 1,
                error: KleeneEntailmentError::DomainMismatch {
                    antecedent_input_arity: 0,
                    consequent_input_arity: 1,
                },
            })
        );
    }
}
