//! BL-14.4 relational Boolean sparsity primitives.
//!
//! This module treats declared pairwise redundancy as a relation rather than
//! collapsing it into independent scalar scores. It retains one deterministic
//! representative (the lowest original index) from each connected redundancy
//! component. It does not infer relations, measure task quality, or claim any
//! performance benefit.

use core::fmt;

use crate::ExactMask;

/// One declared undirected redundancy relation between two candidate indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RedundancyEdge {
    pub left: usize,
    pub right: usize,
}

impl RedundancyEdge {
    #[must_use]
    pub const fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }
}

/// Fail-closed errors for BL-14.4 relational mask construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationalMaskError {
    EmptyCandidateSet,
    SelfRelation { index: usize },
    RelationIndexOutOfRange { index: usize, total: usize },
    DuplicateRelation { left: usize, right: usize },
}

impl fmt::Display for RelationalMaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCandidateSet => write!(f, "BL-14.4 candidate set must be non-empty"),
            Self::SelfRelation { index } => {
                write!(f, "BL-14.4 redundancy relation contains self-edge at {index}")
            }
            Self::RelationIndexOutOfRange { index, total } => write!(
                f,
                "BL-14.4 redundancy relation index {index} is outside candidate set of width {total}"
            ),
            Self::DuplicateRelation { left, right } => write!(
                f,
                "BL-14.4 redundancy relation ({left}, {right}) is duplicated"
            ),
        }
    }
}

impl std::error::Error for RelationalMaskError {}

fn find_root(parents: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parents[root] != root {
        root = parents[root];
    }

    let mut cursor = index;
    while parents[cursor] != cursor {
        let next = parents[cursor];
        parents[cursor] = root;
        cursor = next;
    }
    root
}

/// Materialize a deterministic relational sparsity mask.
///
/// The input relation is undirected. Each connected component represents a
/// declared redundancy group, and exactly one representative is retained from
/// that component: the lowest original candidate index. Isolated candidates
/// form singleton components and are therefore retained.
///
/// The function validates the complete relation before constructing the mask.
/// Duplicate edges are detected independent of endpoint order.
///
/// # Errors
///
/// Returns [`RelationalMaskError::EmptyCandidateSet`] for `total == 0`, rejects
/// self-relations, out-of-range endpoints, and duplicate undirected edges.
pub fn relational_component_mask(
    total: usize,
    edges: &[RedundancyEdge],
) -> Result<ExactMask, RelationalMaskError> {
    if total == 0 {
        return Err(RelationalMaskError::EmptyCandidateSet);
    }

    let mut normalized = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.left >= total {
            return Err(RelationalMaskError::RelationIndexOutOfRange {
                index: edge.left,
                total,
            });
        }
        if edge.right >= total {
            return Err(RelationalMaskError::RelationIndexOutOfRange {
                index: edge.right,
                total,
            });
        }
        if edge.left == edge.right {
            return Err(RelationalMaskError::SelfRelation { index: edge.left });
        }
        let pair = if edge.left < edge.right {
            (edge.left, edge.right)
        } else {
            (edge.right, edge.left)
        };
        if normalized.contains(&pair) {
            return Err(RelationalMaskError::DuplicateRelation {
                left: pair.0,
                right: pair.1,
            });
        }
        normalized.push(pair);
    }

    let mut parents: Vec<usize> = (0..total).collect();
    for (left, right) in normalized {
        let left_root = find_root(&mut parents, left);
        let right_root = find_root(&mut parents, right);
        if left_root != right_root {
            let (lower, higher) = if left_root < right_root {
                (left_root, right_root)
            } else {
                (right_root, left_root)
            };
            parents[higher] = lower;
        }
    }

    let mut representatives = Vec::new();
    for index in 0..total {
        let root = find_root(&mut parents, index);
        if root == index {
            representatives.push(index);
        }
    }

    ExactMask::from_retained_indices(total, &representatives).map_err(|error| match error {
        crate::SparsityError::EmptyMask => RelationalMaskError::EmptyCandidateSet,
        _ => unreachable!("validated BL-14.4 representatives must form a valid ExactMask"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_lowest_index_per_connected_component() {
        let mask = relational_component_mask(
            7,
            &[
                RedundancyEdge::new(2, 1),
                RedundancyEdge::new(2, 3),
                RedundancyEdge::new(6, 5),
            ],
        )
        .unwrap();

        assert_eq!(mask.as_slice(), &[true, true, false, false, true, true, false]);
        assert_eq!(mask.cardinality().retained(), 4);
        assert_eq!(mask.cardinality().total(), 7);
    }

    #[test]
    fn edge_order_does_not_change_the_mask() {
        let first = relational_component_mask(
            5,
            &[RedundancyEdge::new(4, 2), RedundancyEdge::new(2, 1)],
        )
        .unwrap();
        let second = relational_component_mask(
            5,
            &[RedundancyEdge::new(1, 2), RedundancyEdge::new(2, 4)],
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_duplicate_undirected_relations() {
        assert_eq!(
            relational_component_mask(
                4,
                &[RedundancyEdge::new(1, 3), RedundancyEdge::new(3, 1)],
            ),
            Err(RelationalMaskError::DuplicateRelation { left: 1, right: 3 })
        );
    }

    #[test]
    fn rejects_invalid_relations() {
        assert_eq!(
            relational_component_mask(0, &[]),
            Err(RelationalMaskError::EmptyCandidateSet)
        );
        assert_eq!(
            relational_component_mask(3, &[RedundancyEdge::new(1, 1)]),
            Err(RelationalMaskError::SelfRelation { index: 1 })
        );
        assert_eq!(
            relational_component_mask(3, &[RedundancyEdge::new(0, 3)]),
            Err(RelationalMaskError::RelationIndexOutOfRange { index: 3, total: 3 })
        );
    }
}
