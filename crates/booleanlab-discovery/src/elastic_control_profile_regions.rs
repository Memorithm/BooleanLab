//! Compact contiguous occupancy regions for the qualified Elastic profile oracle.
//!
//! Regions preserve the exact minimum/tie mask from the full bounded oracle.
//! They are an offline compression of evidence, not a production runtime policy.

use crate::elastic_control_profile_partition::{
    payloads, ControlProfilePartitionError, DEFAULT_EXHAUSTIVE_MAX_SLOTS,
};

/// Versioned compact-region contract.
pub const ELASTIC_CONTROL_PROFILE_REGIONS_V1: &str =
    "booleanlab.elastic-control-profile-regions@1.0.0";

/// One inclusive present-slot interval with a constant exact minimum/tie mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlProfileRegionV1 {
    present_min: usize,
    present_max: usize,
    minimum_mask: u8,
}

impl ControlProfileRegionV1 {
    #[must_use]
    pub const fn present_min(self) -> usize {
        self.present_min
    }

    #[must_use]
    pub const fn present_max(self) -> usize {
        self.present_max
    }

    #[must_use]
    pub const fn minimum_mask(self) -> u8 {
        self.minimum_mask
    }

    #[must_use]
    pub const fn contains(self, present_slots: usize) -> bool {
        present_slots >= self.present_min && present_slots <= self.present_max
    }
}

/// Compress one qualified slot-count row into contiguous exact regions.
///
/// # Errors
///
/// Returns an error for zero slots or a slot count above the already qualified
/// 4096-slot bounded domain, and propagates exact payload-oracle errors.
pub fn regions_for_slot_count(
    slot_count: usize,
) -> Result<Vec<ControlProfileRegionV1>, ControlProfileRegionError> {
    if slot_count == 0 {
        return Err(ControlProfileRegionError::ZeroSlots);
    }
    if slot_count > DEFAULT_EXHAUSTIVE_MAX_SLOTS {
        return Err(ControlProfileRegionError::OutsideQualifiedDomain {
            slot_count,
            maximum: DEFAULT_EXHAUSTIVE_MAX_SLOTS,
        });
    }

    let mut regions = Vec::new();
    let first_mask = payloads(slot_count, 0)?.exact_minimum_mask();
    let mut current = ControlProfileRegionV1 {
        present_min: 0,
        present_max: 0,
        minimum_mask: first_mask,
    };

    for present_slots in 1..=slot_count {
        let mask = payloads(slot_count, present_slots)?.exact_minimum_mask();
        if mask == current.minimum_mask {
            current.present_max = present_slots;
        } else {
            regions.push(current);
            current = ControlProfileRegionV1 {
                present_min: present_slots,
                present_max: present_slots,
                minimum_mask: mask,
            };
        }
    }
    regions.push(current);
    Ok(regions)
}

/// Resolve one present-slot count through compact regions.
///
/// # Errors
///
/// Returns an error when the occupancy lies outside the slot domain or when
/// region construction fails.
pub fn minimum_mask_from_regions(
    slot_count: usize,
    present_slots: usize,
) -> Result<u8, ControlProfileRegionError> {
    if present_slots > slot_count {
        return Err(ControlProfileRegionError::PresentExceedsSlots {
            present_slots,
            slot_count,
        });
    }
    regions_for_slot_count(slot_count)?
        .into_iter()
        .find(|region| region.contains(present_slots))
        .map(ControlProfileRegionV1::minimum_mask)
        .ok_or(ControlProfileRegionError::CoverageGap {
            slot_count,
            present_slots,
        })
}

/// Fail-closed region construction errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlProfileRegionError {
    ZeroSlots,
    OutsideQualifiedDomain {
        slot_count: usize,
        maximum: usize,
    },
    PresentExceedsSlots {
        present_slots: usize,
        slot_count: usize,
    },
    CoverageGap {
        slot_count: usize,
        present_slots: usize,
    },
    Oracle(ControlProfilePartitionError),
}

impl From<ControlProfilePartitionError> for ControlProfileRegionError {
    fn from(value: ControlProfilePartitionError) -> Self {
        Self::Oracle(value)
    }
}

impl std::fmt::Display for ControlProfileRegionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroSlots => formatter.write_str("profile regions require slot_count > 0"),
            Self::OutsideQualifiedDomain {
                slot_count,
                maximum,
            } => write!(
                formatter,
                "slot count {slot_count} exceeds qualified profile-region maximum {maximum}"
            ),
            Self::PresentExceedsSlots {
                present_slots,
                slot_count,
            } => write!(
                formatter,
                "present slot count {present_slots} exceeds slot count {slot_count}"
            ),
            Self::CoverageGap {
                slot_count,
                present_slots,
            } => write!(
                formatter,
                "profile regions do not cover slots={slot_count} present={present_slots}"
            ),
            Self::Oracle(error) => write!(formatter, "profile oracle failed: {error}"),
        }
    }
}

impl std::error::Error for ControlProfileRegionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_are_contiguous_non_overlapping_and_complete() {
        for slot_count in 1..=DEFAULT_EXHAUSTIVE_MAX_SLOTS {
            let regions = regions_for_slot_count(slot_count).unwrap();
            assert!(!regions.is_empty());
            assert_eq!(regions[0].present_min(), 0);
            assert_eq!(regions.last().unwrap().present_max(), slot_count);
            for pair in regions.windows(2) {
                assert_eq!(pair[0].present_max() + 1, pair[1].present_min());
                assert_ne!(pair[0].minimum_mask(), pair[1].minimum_mask());
            }
        }
    }

    #[test]
    fn compact_regions_replay_full_exact_oracle_over_qualified_domain() {
        for slot_count in 1..=DEFAULT_EXHAUSTIVE_MAX_SLOTS {
            for present_slots in 0..=slot_count {
                let exact = payloads(slot_count, present_slots)
                    .unwrap()
                    .exact_minimum_mask();
                let compact = minimum_mask_from_regions(slot_count, present_slots).unwrap();
                assert_eq!(
                    compact, exact,
                    "slot_count={slot_count}, present_slots={present_slots}"
                );
            }
        }
    }

    #[test]
    fn known_slot_domains_have_small_region_counts() {
        for slot_count in [1, 2, 3, 4, 16, 64, 256, 4096] {
            let regions = regions_for_slot_count(slot_count).unwrap();
            assert!(regions.len() <= 6, "slot_count={slot_count}: {regions:?}");
        }
    }

    #[test]
    fn domain_bounds_fail_closed() {
        assert_eq!(
            regions_for_slot_count(0),
            Err(ControlProfileRegionError::ZeroSlots)
        );
        assert_eq!(
            regions_for_slot_count(DEFAULT_EXHAUSTIVE_MAX_SLOTS + 1),
            Err(ControlProfileRegionError::OutsideQualifiedDomain {
                slot_count: DEFAULT_EXHAUSTIVE_MAX_SLOTS + 1,
                maximum: DEFAULT_EXHAUSTIVE_MAX_SLOTS,
            })
        );
        assert_eq!(
            minimum_mask_from_regions(4, 5),
            Err(ControlProfileRegionError::PresentExceedsSlots {
                present_slots: 5,
                slot_count: 4,
            })
        );
    }
}
