//! Exact structured Boolean masks for BL-14.2.
//!
//! This module lifts a declared static Boolean mask from group indices to a
//! fixed-width element domain. It defines exact structured-mask semantics only;
//! it makes no quality, latency, throughput, traffic, or hardware claim.

use core::fmt;

use crate::{
    sparsity::{ExactMask, SparsityError},
    sparsity_static::{StaticMaskError, static_mask_from_truth_table},
};

/// Errors for exact structured Boolean-mask construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredBooleanMaskError {
    ZeroGroupWidth,
    TotalWidthOverflow { groups: usize, group_width: usize },
    Static(StaticMaskError),
    Sparsity(SparsityError),
}

impl fmt::Display for StructuredBooleanMaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StructuredBooleanMaskError {}

impl From<StaticMaskError> for StructuredBooleanMaskError {
    fn from(error: StaticMaskError) -> Self {
        Self::Static(error)
    }
}

impl From<SparsityError> for StructuredBooleanMaskError {
    fn from(error: SparsityError) -> Self {
        Self::Sparsity(error)
    }
}

/// Expand a static Boolean truth-table decision over fixed-width groups.
///
/// The truth table is evaluated over the **group index**, using the exact
/// complete-period semantics of [`static_mask_from_truth_table`]. A retained
/// group keeps all `group_width` consecutive elements and a dropped group drops
/// all of them. This provides an exact block/channel/head-like grouping primitive
/// without assigning any model-specific meaning to a group.
///
/// # Errors
///
/// Returns [`StructuredBooleanMaskError::ZeroGroupWidth`] when `group_width` is
/// zero, [`StructuredBooleanMaskError::TotalWidthOverflow`] if the expanded
/// element width is not representable by `usize`, a wrapped [`StaticMaskError`]
/// for malformed group-level Boolean semantics, or a wrapped [`SparsityError`]
/// if exact materialization fails.
pub fn structured_group_mask_from_truth_table(
    groups: usize,
    group_width: usize,
    truth_table: &[bool],
) -> Result<ExactMask, StructuredBooleanMaskError> {
    if group_width == 0 {
        return Err(StructuredBooleanMaskError::ZeroGroupWidth);
    }

    let total = groups
        .checked_mul(group_width)
        .ok_or(StructuredBooleanMaskError::TotalWidthOverflow {
            groups,
            group_width,
        })?;
    let group_mask = static_mask_from_truth_table(groups, truth_table)?;

    let mut retained_indices = Vec::with_capacity(
        group_mask
            .cardinality()
            .retained()
            .checked_mul(group_width)
            .ok_or(StructuredBooleanMaskError::TotalWidthOverflow {
                groups,
                group_width,
            })?,
    );

    for (group, &retained) in group_mask.as_slice().iter().enumerate() {
        if retained {
            let start = group * group_width;
            retained_indices.extend(start..start + group_width);
        }
    }

    ExactMask::from_retained_indices(total, &retained_indices).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{StructuredBooleanMaskError, structured_group_mask_from_truth_table};
    use crate::sparsity_static::StaticMaskError;

    #[test]
    fn expands_group_decisions_without_partial_groups() {
        let mask = structured_group_mask_from_truth_table(4, 3, &[true, false]).unwrap();
        assert_eq!(
            mask.as_slice(),
            &[
                true, true, true, false, false, false, true, true, true, false, false, false,
            ]
        );
        assert_eq!(mask.cardinality().retained(), 6);
        assert_eq!(mask.cardinality().total(), 12);
    }

    #[test]
    fn constant_group_function_keeps_or_drops_complete_groups() {
        let keep = structured_group_mask_from_truth_table(3, 2, &[true]).unwrap();
        let drop = structured_group_mask_from_truth_table(3, 2, &[false]).unwrap();
        assert_eq!(keep.cardinality().retained(), 6);
        assert_eq!(drop.cardinality().retained(), 0);
    }

    #[test]
    fn rejects_zero_group_width_fail_closed() {
        assert_eq!(
            structured_group_mask_from_truth_table(4, 0, &[true]),
            Err(StructuredBooleanMaskError::ZeroGroupWidth)
        );
    }

    #[test]
    fn preserves_group_level_complete_period_requirement() {
        assert_eq!(
            structured_group_mask_from_truth_table(3, 2, &[true, false]),
            Err(StructuredBooleanMaskError::Static(
                StaticMaskError::WidthNotMultipleOfTruthTable {
                    total: 3,
                    table_len: 2,
                }
            ))
        );
    }

    #[test]
    fn rejects_unrepresentable_expanded_width_before_allocation() {
        assert_eq!(
            structured_group_mask_from_truth_table(2, usize::MAX, &[true, false]),
            Err(StructuredBooleanMaskError::TotalWidthOverflow {
                groups: 2,
                group_width: usize::MAX,
            })
        );
    }
}
