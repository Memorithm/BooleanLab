//! Necessary-invariant screen for bounded CCZ-equivalence candidates.
//!
//! CCZ-equivalent vectorial Boolean maps have the same differential spectrum
//! and the same extended absolute Walsh spectrum. A mismatch therefore rules
//! out CCZ-equivalence. Matching spectra are only an inconclusive necessary
//! condition and are never reported as an equivalence proof.

use core::fmt;

use crate::{
    DEFAULT_VECTORIAL_MAX_WORK, VectorialMetricsError,
    vectorial_differential_spectrum_with_work_limit, vectorial_walsh_spectrum_with_work_limit,
};

/// First exact invariant that rules out CCZ-equivalence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorialCczMismatch {
    /// The exact differential-spectrum histograms differ.
    DifferentialSpectrum,
    /// The exact absolute non-zero-component Walsh-spectrum histograms differ.
    AbsoluteExtendedWalshSpectrum,
}

/// Outcome of the bounded necessary-invariant screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorialCczScreen {
    /// At least one CCZ invariant differs, so the maps cannot be CCZ-equivalent.
    RuledOut(VectorialCczMismatch),
    /// The implemented necessary invariants match; equivalence remains unknown.
    Inconclusive,
}

/// Side-specific failure while computing an exact invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorialCczScreenError {
    /// Exact metric computation failed for the left table.
    Left(VectorialMetricsError),
    /// Exact metric computation failed for the right table.
    Right(VectorialMetricsError),
}

impl fmt::Display for VectorialCczScreenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(error) => {
                write!(formatter, "left CCZ invariant computation failed: {error}")
            }
            Self::Right(error) => {
                write!(formatter, "right CCZ invariant computation failed: {error}")
            }
        }
    }
}

impl std::error::Error for VectorialCczScreenError {}

/// Screen two declared `F_2^n -> F_2^m` truth tables with exact necessary CCZ
/// invariants under the default per-metric work budget.
///
/// # Errors
///
/// Returns [`VectorialCczScreenError`] when either table is malformed, a value
/// exceeds the declared output width, exact work exceeds the configured metric
/// budget, or arithmetic/allocation fails. Exhaustion is a non-result.
pub fn vectorial_ccz_invariant_screen(
    left: &[u16],
    right: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialCczScreen, VectorialCczScreenError> {
    vectorial_ccz_invariant_screen_with_work_limit(
        left,
        right,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

/// Screen two declared vectorial truth tables with an explicit work limit for
/// each individual exact spectrum computation.
///
/// The function stops as soon as an invariant mismatch is found. The Walsh
/// spectrum implementation enumerates every non-zero output mask. For fixed
/// `n` and `m`, the omitted zero-output-mask Walsh slice is identical for every
/// map (`2^n` at input mask zero and zero elsewhere), so equality of the stored
/// absolute spectrum is equivalent to equality of the full extended absolute
/// Walsh multiset.
///
/// A returned [`VectorialCczScreen::Inconclusive`] means only that these two
/// necessary invariants match. It does not establish CCZ- or EA-equivalence,
/// cryptographic suitability, prior-art status, novelty, or performance.
///
/// # Errors
///
/// Returns [`VectorialCczScreenError`] under the same conditions as
/// [`vectorial_ccz_invariant_screen`].
pub fn vectorial_ccz_invariant_screen_with_work_limit(
    left: &[u16],
    right: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work_per_metric: u128,
) -> Result<VectorialCczScreen, VectorialCczScreenError> {
    let left_differential = vectorial_differential_spectrum_with_work_limit(
        left,
        input_bits,
        output_bits,
        max_work_per_metric,
    )
    .map_err(VectorialCczScreenError::Left)?;
    let right_differential = vectorial_differential_spectrum_with_work_limit(
        right,
        input_bits,
        output_bits,
        max_work_per_metric,
    )
    .map_err(VectorialCczScreenError::Right)?;

    if left_differential.spectrum != right_differential.spectrum {
        return Ok(VectorialCczScreen::RuledOut(
            VectorialCczMismatch::DifferentialSpectrum,
        ));
    }

    let left_walsh = vectorial_walsh_spectrum_with_work_limit(
        left,
        input_bits,
        output_bits,
        max_work_per_metric,
    )
    .map_err(VectorialCczScreenError::Left)?;
    let right_walsh = vectorial_walsh_spectrum_with_work_limit(
        right,
        input_bits,
        output_bits,
        max_work_per_metric,
    )
    .map_err(VectorialCczScreenError::Right)?;

    if left_walsh.spectrum != right_walsh.spectrum {
        return Ok(VectorialCczScreen::RuledOut(
            VectorialCczMismatch::AbsoluteExtendedWalshSpectrum,
        ));
    }

    Ok(VectorialCczScreen::Inconclusive)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESENT: [u16; 16] = [
        0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
    ];

    #[test]
    fn differential_mismatch_rules_out_identity_vs_present() {
        let identity: Vec<u16> = (0u16..16).collect();
        assert_eq!(
            vectorial_ccz_invariant_screen(&identity, &PRESENT, 4, 4).unwrap(),
            VectorialCczScreen::RuledOut(VectorialCczMismatch::DifferentialSpectrum)
        );
    }

    #[test]
    fn output_translation_matches_invariants_but_remains_inconclusive() {
        let translated: Vec<u16> = PRESENT.iter().map(|value| value ^ 0xF).collect();
        assert_eq!(
            vectorial_ccz_invariant_screen(&PRESENT, &translated, 4, 4).unwrap(),
            VectorialCczScreen::Inconclusive
        );
    }

    #[test]
    fn walsh_mismatch_can_rule_out_after_differential_match() {
        let left = [15, 12, 7, 5, 6, 14, 8, 13, 0, 2, 13, 10, 3, 2, 6, 15];
        let right = [6, 6, 11, 7, 13, 12, 8, 15, 10, 0, 6, 1, 15, 0, 6, 7];
        assert_eq!(
            vectorial_ccz_invariant_screen(&left, &right, 4, 4).unwrap(),
            VectorialCczScreen::RuledOut(VectorialCczMismatch::AbsoluteExtendedWalshSpectrum)
        );
    }

    #[test]
    fn malformed_or_over_budget_inputs_are_non_results() {
        assert!(matches!(
            vectorial_ccz_invariant_screen(&[0, 1, 2], &[0, 1, 2, 3], 2, 2),
            Err(VectorialCczScreenError::Left(
                VectorialMetricsError::TableLengthMismatch { .. }
            ))
        ));

        let identity: Vec<u16> = (0u16..16).collect();
        assert!(matches!(
            vectorial_ccz_invariant_screen_with_work_limit(&identity, &identity, 4, 4, 1),
            Err(VectorialCczScreenError::Left(
                VectorialMetricsError::WorkLimitExceeded { .. }
            ))
        ));
    }
}
