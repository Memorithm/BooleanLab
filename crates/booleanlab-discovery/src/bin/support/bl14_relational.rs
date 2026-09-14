//! BL-14.4.1 development pilot for relation-aware Boolean sparsity.
//!
//! The pilot deliberately reuses the frozen BL-14.2.2 trained-readout workload.
//! Pairwise relations are derived only from TRAIN activation-state agreement,
//! SEARCH chooses a prefix of the preregistered relation ranking, and every
//! selected mask is frozen before any non-final VALIDATION batch is built.
//! A declared relation is a candidate redundancy signal, not proof that two
//! units are functionally interchangeable. Reference operation counters are not
//! elapsed-time, memory-traffic, energy, or hardware-performance evidence.

#[cfg(test)]
use std::cmp::Ordering;
use std::collections::BTreeSet;

use booleanlab_core::{ExactMask, RedundancyEdge, relational_component_mask};

#[cfg(test)]
use super::Work;
use super::{
    Batch, FrozenTrial, KEEP, Metrics, Result, Split, Trial, UNITS, features, generate, mask_code,
    prepare, require_batch, score, trials,
};

const PAIRS: usize = UNITS * (UNITS - 1) / 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RankedEdge {
    edge: RedundancyEdge,
    activation_agreements: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct RelationalCandidate {
    edge_prefix: usize,
    mask: ExactMask,
    search: Metrics,
}

#[derive(Debug)]
struct RelationalStudy {
    base: FrozenTrial,
    ranked_edges: Vec<RankedEdge>,
    candidates: Vec<RelationalCandidate>,
    selected: usize,
    relation_pair_tests: usize,
    search_candidate_sample_evaluations: usize,
}

fn rank_relations(batch: &Batch) -> Result<(Vec<RankedEdge>, usize)> {
    require_batch(batch, batch.trial, Split::Train)?;
    let mut agreements = [[0usize; UNITS]; UNITS];
    let mut pair_tests = 0usize;
    for sample in &batch.samples {
        let values = features(&sample.input)?;
        for left in 0..UNITS {
            for right in (left + 1)..UNITS {
                pair_tests = pair_tests
                    .checked_add(1)
                    .ok_or("BL-14.4 relation pair-test count overflow")?;
                if (values[left] > 0.0) == (values[right] > 0.0) {
                    agreements[left][right] = agreements[left][right]
                        .checked_add(1)
                        .ok_or("BL-14.4 activation-agreement count overflow")?;
                }
            }
        }
    }

    let mut ranked = Vec::with_capacity(PAIRS);
    for (left, row) in agreements.iter().enumerate() {
        for (right, &activation_agreements) in row.iter().enumerate().skip(left + 1) {
            ranked.push(RankedEdge {
                edge: RedundancyEdge::new(left, right),
                activation_agreements,
            });
        }
    }
    ranked.sort_by(|left, right| {
        right
            .activation_agreements
            .cmp(&left.activation_agreements)
            .then_with(|| left.edge.left.cmp(&right.edge.left))
            .then_with(|| left.edge.right.cmp(&right.edge.right))
    });
    if ranked.len() != PAIRS {
        return Err("BL-14.4 relation ranking is incomplete".into());
    }
    Ok((ranked, pair_tests))
}

fn mask_for_prefix(ranked: &[RankedEdge], prefix: usize) -> Result<ExactMask> {
    if prefix == 0 || prefix > ranked.len() {
        return Err("BL-14.4 relation prefix is outside the declared ranking".into());
    }
    let edges: Vec<RedundancyEdge> = ranked[..prefix].iter().map(|entry| entry.edge).collect();
    Ok(relational_component_mask(UNITS, &edges)?)
}

fn candidate_family(
    base: &FrozenTrial,
    search: &Batch,
    ranked: &[RankedEdge],
) -> Result<Vec<RelationalCandidate>> {
    require_batch(search, base.trial, Split::Search)?;
    let mut seen_masks = BTreeSet::new();
    let mut candidates = Vec::new();
    for edge_prefix in 1..=ranked.len() {
        let mask = mask_for_prefix(ranked, edge_prefix)?;
        if mask.cardinality().retained() != KEEP || !seen_masks.insert(mask_code(&mask)) {
            continue;
        }
        candidates.push(RelationalCandidate {
            edge_prefix,
            search: score(&base.weights, search, Some(&mask))?,
            mask,
        });
    }
    if candidates.is_empty() {
        return Err("BL-14.4 relation ranking produced no exact 4/8 candidate".into());
    }
    Ok(candidates)
}

fn best_candidate(candidates: &[RelationalCandidate]) -> Result<usize> {
    candidates
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            left.search
                .task_mse
                .total_cmp(&right.search.task_mse)
                .then_with(|| left.edge_prefix.cmp(&right.edge_prefix))
                .then_with(|| mask_code(&left.mask).cmp(&mask_code(&right.mask)))
        })
        .map(|(index, _)| index)
        .ok_or_else(|| "BL-14.4 has no relation-aware candidate".into())
}

fn prepare_relational(trial: Trial) -> Result<RelationalStudy> {
    let base = prepare(trial)?;
    let train = generate(trial, Split::Train)?;
    let search = generate(trial, Split::Search)?;
    let (ranked_edges, relation_pair_tests) = rank_relations(&train)?;
    let candidates = candidate_family(&base, &search, &ranked_edges)?;
    let selected = best_candidate(&candidates)?;
    let search_candidate_sample_evaluations = candidates
        .len()
        .checked_mul(search.samples.len())
        .ok_or("BL-14.4 SEARCH evaluation budget overflow")?;
    Ok(RelationalStudy {
        base,
        ranked_edges,
        candidates,
        selected,
        relation_pair_tests,
        search_candidate_sample_evaluations,
    })
}

impl RelationalStudy {
    fn selected(&self) -> Result<&RelationalCandidate> {
        self.candidates
            .get(self.selected)
            .ok_or_else(|| "BL-14.4 frozen candidate index is out of range".into())
    }

    fn verify_frozen_relation(&self) -> Result<()> {
        let train = generate(self.base.trial, Split::Train)?;
        let (ranked, pair_tests) = rank_relations(&train)?;
        if ranked != self.ranked_edges || pair_tests != self.relation_pair_tests {
            return Err("BL-14.4 frozen TRAIN relation ranking changed".into());
        }
        let selected = self.selected()?;
        let reconstructed = mask_for_prefix(&ranked, selected.edge_prefix)?;
        if reconstructed != selected.mask || reconstructed.cardinality().retained() != KEEP {
            return Err("BL-14.4 frozen relation no longer materializes its exact 4/8 mask".into());
        }
        Ok(())
    }

    fn metrics(&self, split: Split) -> Result<Vec<(String, Metrics)>> {
        if !matches!(split, Split::Search | Split::Validation) {
            return Err("BL-14.4 may only score SEARCH or non-final VALIDATION".into());
        }
        self.verify_frozen_relation()?;
        let batch = generate(self.base.trial, split)?;
        require_batch(&batch, self.base.trial, split)?;
        let mut rows = Vec::new();
        rows.push((
            "dense".to_owned(),
            if split == Split::Search {
                self.base.dense_search.clone()
            } else {
                score(&self.base.weights, &batch, None)?
            },
        ));
        for choice in &self.base.baselines {
            rows.push((
                choice.id.clone(),
                if split == Split::Search {
                    choice.search.clone()
                } else {
                    score(&self.base.weights, &batch, Some(&choice.mask))?
                },
            ));
        }
        let static_index = *self
            .base
            .selected
            .first()
            .ok_or("BL-14.4 has no frozen BL-14.2.2 Boolean baseline")?;
        let static_choice = self
            .base
            .boolean
            .get(static_index)
            .ok_or("BL-14.4 frozen static Boolean baseline is out of range")?;
        rows.push((
            "boolean_static_best".to_owned(),
            if split == Split::Search {
                static_choice.search.clone()
            } else {
                score(&self.base.weights, &batch, Some(&static_choice.mask))?
            },
        ));
        let selected = self.selected()?;
        rows.push((
            "boolean_relational_activation_agreement".to_owned(),
            if split == Split::Search {
                selected.search.clone()
            } else {
                score(&self.base.weights, &batch, Some(&selected.mask))?
            },
        ));
        Ok(rows)
    }
}

fn emit(trial: Trial, stage: &str, id: &str, metrics: &Metrics) {
    println!(
        "{}\t{}\t{stage}\t{id}\t{:.17e}\t{:.17e}\t{}\t{}\t{}",
        trial.regime.name(),
        trial.seed,
        metrics.task_mse,
        metrics.reconstruction_mse,
        metrics.work.multiplications,
        metrics.work.relus,
        metrics.work.mask_tests,
    );
}

/// Execute the BL-14.4.1 non-final relational-sparsity development pilot.
pub(super) fn run() -> Result<()> {
    // Freeze every trained model, relation ranking, prefix choice, Boolean
    // baseline and random control before constructing any VALIDATION batch.
    let frozen: Vec<RelationalStudy> = trials()
        .into_iter()
        .map(prepare_relational)
        .collect::<Result<_>>()?;
    println!(
        "# schema=bl14.relational-activation-agreement.v1; phase=NUMERICAL_DEVELOPMENT; trials=12"
    );
    println!("# relation=train-only equality of ReLU activation state (>0 versus ==0)");
    println!(
        "# relation edges rank by agreement count descending, then endpoint indices ascending"
    );
    println!("# SEARCH chooses only among relation prefixes that retain exactly 4/8 units");
    println!("# connected-component representative tie-break=lowest original unit index");
    println!("# relation is a candidate redundancy signal, not proof of functional equivalence");
    println!("# selection costs are reported separately from frozen sparse inference work");
    println!("# no hardware timing, memory traffic, energy, final holdout or speedup claim");
    println!("regime\tseed\tstage\tpolicy\ttask_mse\treconstruction_mse\tmuls\trelus\tmask_tests");
    for study in &frozen {
        let selected = study.selected()?;
        let boundary = study
            .ranked_edges
            .get(selected.edge_prefix - 1)
            .ok_or("BL-14.4 selected relation boundary is absent")?;
        println!(
            "# trial={}:{}; relation_pair_tests={}; unique_4_of_8_candidates={}; search_candidate_sample_evaluations={}; selected_edge_prefix={}; selected_mask={}; boundary_agreements={}; boundary_edge=({}, {})",
            study.base.trial.regime.name(),
            study.base.trial.seed,
            study.relation_pair_tests,
            study.candidates.len(),
            study.search_candidate_sample_evaluations,
            selected.edge_prefix,
            mask_code(&selected.mask),
            boundary.activation_agreements,
            boundary.edge.left,
            boundary.edge.right,
        );
        for (id, metrics) in study.metrics(Split::Search)? {
            emit(study.base.trial, "SEARCH", &id, &metrics);
        }
    }
    println!("# ALL_RELATIONAL_CONTROLLERS_FROZEN_BEFORE_VALIDATION");
    for study in &frozen {
        for (id, metrics) in study.metrics(Split::Validation)? {
            emit(study.base.trial, "VALIDATION", &id, &metrics);
        }
    }
    println!("# COMPLETE_TRIALS=12");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_ranking_is_complete_unique_and_train_only() {
        for trial in trials() {
            let train = generate(trial, Split::Train).unwrap();
            let (first, pair_tests) = rank_relations(&train).unwrap();
            let (second, replay_tests) = rank_relations(&train).unwrap();
            assert_eq!(first, second);
            assert_eq!(first.len(), PAIRS);
            assert_eq!(pair_tests, 128 * PAIRS);
            assert_eq!(pair_tests, replay_tests);
            let unique: BTreeSet<(usize, usize)> = first
                .iter()
                .map(|entry| (entry.edge.left, entry.edge.right))
                .collect();
            assert_eq!(unique.len(), PAIRS);
            assert!(first.iter().all(|entry| entry.edge.left < entry.edge.right));
        }
    }

    #[test]
    fn search_family_is_exact_density_and_selected_deterministically() {
        for trial in trials() {
            let study = prepare_relational(trial).unwrap();
            let selected = study.selected().unwrap();
            assert_eq!(selected.mask.cardinality().retained(), KEEP);
            assert!(
                study
                    .candidates
                    .iter()
                    .all(|candidate| candidate.mask.cardinality().retained() == KEEP)
            );
            let expected = best_candidate(&study.candidates).unwrap();
            assert_eq!(study.selected, expected);
            study.verify_frozen_relation().unwrap();
        }
    }

    #[test]
    fn relational_policy_keeps_selection_cost_separate_from_sparse_work() {
        let study = prepare_relational(trials()[0]).unwrap();
        assert_eq!(study.relation_pair_tests, 128 * PAIRS);
        assert_eq!(
            study.search_candidate_sample_evaluations,
            study.candidates.len() * 64
        );
        for split in [Split::Search, Split::Validation] {
            let rows = study.metrics(split).unwrap();
            let relational = rows
                .iter()
                .find(|(id, _)| id == "boolean_relational_activation_agreement")
                .unwrap();
            assert_eq!(
                relational.1.work,
                Work {
                    multiplications: 1280,
                    relus: 256,
                    mask_tests: 512,
                }
            );
            let dense = rows.iter().find(|(id, _)| id == "dense").unwrap();
            assert_eq!(dense.1.work.multiplications, 2560);
            assert_eq!(dense.1.work.relus, 512);
            assert_eq!(dense.1.work.mask_tests, 0);
        }
    }

    #[test]
    fn validation_does_not_change_frozen_relation_or_choice() {
        let study = prepare_relational(trials()[0]).unwrap();
        let selected = study.selected;
        let snapshot = format!("{study:?}");
        let first = study.metrics(Split::Validation).unwrap();
        assert_eq!(first, study.metrics(Split::Validation).unwrap());
        assert_eq!(selected, study.selected);
        assert_eq!(snapshot, format!("{study:?}"));
    }

    #[test]
    fn tie_order_is_declared_and_stable() {
        let trial = trials()[0];
        let train = generate(trial, Split::Train).unwrap();
        let (ranked, _) = rank_relations(&train).unwrap();
        for pair in ranked.windows(2) {
            let ordering = pair[1]
                .activation_agreements
                .cmp(&pair[0].activation_agreements);
            assert_ne!(ordering, Ordering::Greater);
            if pair[0].activation_agreements == pair[1].activation_agreements {
                assert!(
                    (pair[0].edge.left, pair[0].edge.right)
                        < (pair[1].edge.left, pair[1].edge.right)
                );
            }
        }
    }
}
