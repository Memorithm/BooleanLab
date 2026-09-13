//! Evidence accounting for BL-4.6 Boolean attention systems qualification.
//!
//! This module deliberately separates logical work accounting from measured
//! hardware evidence. A reduction in logical pairs or bytes is not a speedup
//! claim. Timing comparisons are admitted only when the candidate and dense
//! baseline use the same declared timing source.

use core::fmt;

/// Provenance of byte-traffic evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrafficEvidenceKind {
    /// Exact bytes implied by a declared software access model. This is not a
    /// physical DRAM/cache transaction measurement.
    LogicalAccounting,
    /// Bytes reported by a declared hardware/backend counter.
    HardwareCounter,
}

/// Provenance of timing evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimingEvidenceKind {
    /// Host wall-clock timing around a declared execution interval.
    HostWallClock,
    /// Backend/device timestamp timing around a declared execution interval.
    DeviceTimestamp,
}

/// Exact integer work/traffic evidence for one candidate execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttentionWorkEvidence {
    /// Number of attention pairs in the matched dense problem.
    pub dense_pairs: u64,
    /// Number of pairs admitted by the Boolean control plane.
    pub admitted_pairs: u64,
    /// Number of exact Q/K evaluations actually executed by the candidate.
    pub exact_qk_evaluations: u64,
    /// Dense K/V bytes under the declared traffic evidence kind.
    pub dense_kv_bytes: u64,
    /// Candidate numerical K/V bytes under the same evidence kind.
    pub candidate_kv_bytes: u64,
    /// Boolean metadata bytes consumed by the candidate.
    pub boolean_metadata_bytes: u64,
    /// Provenance for both dense and candidate numerical K/V byte counts.
    pub traffic_kind: TrafficEvidenceKind,
}

impl AttentionWorkEvidence {
    /// Validate subset/accounting invariants without interpreting them as a
    /// performance result.
    ///
    /// # Errors
    ///
    /// Returns an error when candidate work exceeds the matched dense problem
    /// or when numerical K/V traffic exceeds the declared dense baseline.
    pub fn validate(self) -> Result<(), SystemsEvidenceError> {
        if self.dense_pairs == 0 {
            return Err(SystemsEvidenceError::ZeroDensePairs);
        }
        if self.admitted_pairs > self.dense_pairs {
            return Err(SystemsEvidenceError::AdmittedPairsExceedDense {
                admitted: self.admitted_pairs,
                dense: self.dense_pairs,
            });
        }
        if self.exact_qk_evaluations > self.admitted_pairs {
            return Err(SystemsEvidenceError::ExactWorkExceedsAdmission {
                exact: self.exact_qk_evaluations,
                admitted: self.admitted_pairs,
            });
        }
        if self.candidate_kv_bytes > self.dense_kv_bytes {
            return Err(SystemsEvidenceError::CandidateKvBytesExceedDense {
                candidate: self.candidate_kv_bytes,
                dense: self.dense_kv_bytes,
            });
        }
        Ok(())
    }

    /// Exact number of dense attention pairs rejected before exact Q/K work.
    ///
    /// # Errors
    ///
    /// Validates the evidence first so subtraction cannot hide malformed input.
    pub fn rejected_pairs(self) -> Result<u64, SystemsEvidenceError> {
        self.validate()?;
        Ok(self.dense_pairs - self.admitted_pairs)
    }

    /// Exact numerical K/V bytes absent from the candidate under the declared
    /// traffic evidence kind.
    ///
    /// For [`TrafficEvidenceKind::LogicalAccounting`] this remains a logical
    /// byte reduction, not evidence that physical memory traffic fell by the
    /// same amount.
    pub fn kv_bytes_not_consumed(self) -> Result<u64, SystemsEvidenceError> {
        self.validate()?;
        Ok(self.dense_kv_bytes - self.candidate_kv_bytes)
    }
}

/// Timing for one matched dense/candidate comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttentionTimingEvidence {
    /// One timing provenance shared by every interval in this record.
    pub timing_kind: TimingEvidenceKind,
    /// Boolean routing/signature/control-plane interval.
    pub boolean_front_end_ns: u64,
    /// Exact numerical attention work performed for admitted survivors.
    pub exact_survivor_ns: u64,
    /// Dispatch, synchronization and other explicitly attributable candidate
    /// overhead not included in the two intervals above.
    pub dispatch_sync_ns: u64,
    /// Matched dense baseline end-to-end interval measured with `timing_kind`.
    pub dense_baseline_ns: u64,
}

impl AttentionTimingEvidence {
    /// Candidate end-to-end interval represented by this decomposed record.
    ///
    /// # Errors
    ///
    /// Returns an explicit overflow error rather than wrapping durations.
    pub fn candidate_total_ns(self) -> Result<u64, SystemsEvidenceError> {
        self.boolean_front_end_ns
            .checked_add(self.exact_survivor_ns)
            .and_then(|value| value.checked_add(self.dispatch_sync_ns))
            .ok_or(SystemsEvidenceError::TimingOverflow)
    }

    /// Validate that both matched paths have non-zero observable duration.
    pub fn validate(self) -> Result<(), SystemsEvidenceError> {
        let candidate = self.candidate_total_ns()?;
        if candidate == 0 || self.dense_baseline_ns == 0 {
            return Err(SystemsEvidenceError::ZeroTimingInterval);
        }
        Ok(())
    }

    /// Signed nanosecond delta `candidate - dense` without converting the
    /// evidence into a rounded floating-point speedup ratio.
    pub fn candidate_minus_dense_ns(self) -> Result<i128, SystemsEvidenceError> {
        self.validate()?;
        Ok(i128::from(self.candidate_total_ns()?) - i128::from(self.dense_baseline_ns))
    }
}

/// Complete BL-4.6 evidence record for one matched workload observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttentionSystemsEvidence {
    pub work: AttentionWorkEvidence,
    pub timing: AttentionTimingEvidence,
}

impl AttentionSystemsEvidence {
    /// Validate the two independent evidence planes.
    pub fn validate(self) -> Result<(), SystemsEvidenceError> {
        self.work.validate()?;
        self.timing.validate()
    }
}

/// Fail-closed BL-4.6 accounting errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemsEvidenceError {
    ZeroDensePairs,
    AdmittedPairsExceedDense { admitted: u64, dense: u64 },
    ExactWorkExceedsAdmission { exact: u64, admitted: u64 },
    CandidateKvBytesExceedDense { candidate: u64, dense: u64 },
    TimingOverflow,
    ZeroTimingInterval,
}

impl fmt::Display for SystemsEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SystemsEvidenceError {}

#[cfg(test)]
mod tests {
    use super::{
        AttentionSystemsEvidence, AttentionTimingEvidence, AttentionWorkEvidence,
        SystemsEvidenceError, TimingEvidenceKind, TrafficEvidenceKind,
    };

    fn work() -> AttentionWorkEvidence {
        AttentionWorkEvidence {
            dense_pairs: 1_024,
            admitted_pairs: 256,
            exact_qk_evaluations: 256,
            dense_kv_bytes: 65_536,
            candidate_kv_bytes: 16_384,
            boolean_metadata_bytes: 512,
            traffic_kind: TrafficEvidenceKind::LogicalAccounting,
        }
    }

    fn timing() -> AttentionTimingEvidence {
        AttentionTimingEvidence {
            timing_kind: TimingEvidenceKind::DeviceTimestamp,
            boolean_front_end_ns: 800,
            exact_survivor_ns: 2_000,
            dispatch_sync_ns: 200,
            dense_baseline_ns: 4_000,
        }
    }

    #[test]
    fn preserves_exact_work_and_traffic_deltas() {
        let evidence = work();
        evidence.validate().unwrap();
        assert_eq!(evidence.rejected_pairs().unwrap(), 768);
        assert_eq!(evidence.kv_bytes_not_consumed().unwrap(), 49_152);
        assert_eq!(evidence.boolean_metadata_bytes, 512);
        assert_eq!(
            evidence.traffic_kind,
            TrafficEvidenceKind::LogicalAccounting
        );
    }

    #[test]
    fn timing_keeps_components_and_signed_delta_exact() {
        let evidence = timing();
        assert_eq!(evidence.candidate_total_ns().unwrap(), 3_000);
        assert_eq!(evidence.candidate_minus_dense_ns().unwrap(), -1_000);
    }

    #[test]
    fn complete_record_validates_both_evidence_planes() {
        AttentionSystemsEvidence {
            work: work(),
            timing: timing(),
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn work_accounting_fails_closed_on_impossible_subsets() {
        let mut evidence = work();
        evidence.admitted_pairs = 1_025;
        assert_eq!(
            evidence.validate(),
            Err(SystemsEvidenceError::AdmittedPairsExceedDense {
                admitted: 1_025,
                dense: 1_024,
            })
        );

        let mut evidence = work();
        evidence.exact_qk_evaluations = 257;
        assert_eq!(
            evidence.validate(),
            Err(SystemsEvidenceError::ExactWorkExceedsAdmission {
                exact: 257,
                admitted: 256,
            })
        );

        let mut evidence = work();
        evidence.candidate_kv_bytes = 65_537;
        assert_eq!(
            evidence.validate(),
            Err(SystemsEvidenceError::CandidateKvBytesExceedDense {
                candidate: 65_537,
                dense: 65_536,
            })
        );
    }

    #[test]
    fn timing_rejects_zero_intervals_and_overflow() {
        let mut evidence = timing();
        evidence.boolean_front_end_ns = 0;
        evidence.exact_survivor_ns = 0;
        evidence.dispatch_sync_ns = 0;
        assert_eq!(
            evidence.validate(),
            Err(SystemsEvidenceError::ZeroTimingInterval)
        );

        let evidence = AttentionTimingEvidence {
            timing_kind: TimingEvidenceKind::HostWallClock,
            boolean_front_end_ns: u64::MAX,
            exact_survivor_ns: 1,
            dispatch_sync_ns: 0,
            dense_baseline_ns: 1,
        };
        assert_eq!(
            evidence.candidate_total_ns(),
            Err(SystemsEvidenceError::TimingOverflow)
        );
    }
}
