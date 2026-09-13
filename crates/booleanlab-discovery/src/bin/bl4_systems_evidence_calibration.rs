use booleanlab_core::{
    AttentionSystemsEvidence, AttentionTimingEvidence, AttentionWorkEvidence, TimingEvidenceKind,
    TrafficEvidenceKind,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Synthetic calibration only. These values validate the BL-4.6 evidence
    // contract and deliberately do not represent a measured FLAT result.
    let evidence = AttentionSystemsEvidence {
        work: AttentionWorkEvidence {
            dense_pairs: 4_096,
            admitted_pairs: 1_024,
            exact_qk_evaluations: 1_024,
            dense_kv_bytes: 262_144,
            candidate_kv_bytes: 65_536,
            boolean_metadata_bytes: 2_048,
            traffic_kind: TrafficEvidenceKind::LogicalAccounting,
        },
        timing: AttentionTimingEvidence {
            timing_kind: TimingEvidenceKind::HostWallClock,
            boolean_front_end_ns: 1_000,
            exact_survivor_ns: 3_000,
            dispatch_sync_ns: 700,
            dense_baseline_ns: 4_000,
        },
    };

    evidence.validate()?;

    let rejected_pairs = evidence.work.rejected_pairs()?;
    let kv_bytes_not_consumed = evidence.work.kv_bytes_not_consumed()?;
    let candidate_total_ns = evidence.timing.candidate_total_ns()?;
    let candidate_minus_dense_ns = evidence.timing.candidate_minus_dense_ns()?;

    if rejected_pairs != 3_072
        || kv_bytes_not_consumed != 196_608
        || candidate_total_ns != 4_700
        || candidate_minus_dense_ns != 700
    {
        return Err(format!(
            "unexpected BL-4.6 calibration: rejected_pairs={rejected_pairs}, kv_bytes_not_consumed={kv_bytes_not_consumed}, candidate_total_ns={candidate_total_ns}, candidate_minus_dense_ns={candidate_minus_dense_ns}"
        )
        .into());
    }

    println!("BL-4.6.1 systems-evidence contract calibration (SYNTHETIC; NOT A FLAT PERFORMANCE RESULT)");
    println!("field\tvalue");
    println!("dense_pairs\t{}", evidence.work.dense_pairs);
    println!("admitted_pairs\t{}", evidence.work.admitted_pairs);
    println!("rejected_pairs\t{rejected_pairs}");
    println!("exact_qk_evaluations\t{}", evidence.work.exact_qk_evaluations);
    println!("dense_kv_bytes\t{}", evidence.work.dense_kv_bytes);
    println!("candidate_kv_bytes\t{}", evidence.work.candidate_kv_bytes);
    println!("kv_bytes_not_consumed\t{kv_bytes_not_consumed}");
    println!("boolean_metadata_bytes\t{}", evidence.work.boolean_metadata_bytes);
    println!("traffic_kind\t{:?}", evidence.work.traffic_kind);
    println!("timing_kind\t{:?}", evidence.timing.timing_kind);
    println!("boolean_front_end_ns\t{}", evidence.timing.boolean_front_end_ns);
    println!("exact_survivor_ns\t{}", evidence.timing.exact_survivor_ns);
    println!("dispatch_sync_ns\t{}", evidence.timing.dispatch_sync_ns);
    println!("candidate_total_ns\t{candidate_total_ns}");
    println!("dense_baseline_ns\t{}", evidence.timing.dense_baseline_ns);
    println!("candidate_minus_dense_ns\t{candidate_minus_dense_ns}");

    Ok(())
}
