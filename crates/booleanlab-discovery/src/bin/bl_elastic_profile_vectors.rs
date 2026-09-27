#![forbid(unsafe_code)]

//! Deterministic BL-BE5 export of exact Elastic control-profile partition vectors.

use booleanlab_discovery::elastic_control_profile_partition::{
    SLHA_CONTROL_PROFILE_SOURCE_REVISION, payloads, reduced_predicates,
};
use std::env;
use std::process::ExitCode;

const VECTOR_SCHEMA: &str = "booleanlab.elastic-control-profile-vectors@1.0.0";
const DEFAULT_MAX_SLOTS: usize = 256;
const MAX_EXPORT_SLOTS: usize = 4096;

fn parse_max_slots_from<I>(args: I) -> Result<usize, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let max_slots = match args.next() {
        None => DEFAULT_MAX_SLOTS,
        Some(value) => value
            .parse::<usize>()
            .map_err(|error| format!("invalid max_slots: {error}"))?,
    };
    if args.next().is_some() {
        return Err("too many arguments".to_owned());
    }
    if max_slots == 0 || max_slots > MAX_EXPORT_SLOTS {
        return Err(format!(
            "max_slots must be in 1..={MAX_EXPORT_SLOTS}, observed {max_slots}"
        ));
    }
    Ok(max_slots)
}

fn parse_max_slots() -> Result<usize, String> {
    parse_max_slots_from(env::args().skip(1))
}

fn expected_rows(max_slots: usize) -> Result<u64, String> {
    let n = u64::try_from(max_slots).map_err(|_| "max_slots does not fit u64")?;
    n.checked_mul(n + 3)
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| "row count overflow".to_owned())
}

fn run(max_slots: usize) -> Result<(), String> {
    println!("# schema={VECTOR_SCHEMA}");
    println!("# source_revision={SLHA_CONTROL_PROFILE_SOURCE_REVISION}");
    println!("# max_slots={max_slots}");
    println!(
        "slot_count\tpresent_slots\tsparse_w512_bits\tdense_w128_bits\thybrid_w64_boolean_bits\texact_minimum_mask\tsparse_le_dense\tsparse_le_hybrid\thybrid_le_dense"
    );

    let mut rows = 0_u64;
    for slot_count in 1..=max_slots {
        for present_slots in 0..=slot_count {
            let costs = payloads(slot_count, present_slots).map_err(|error| error.to_string())?;
            let reduced =
                reduced_predicates(slot_count, present_slots).map_err(|error| error.to_string())?;
            println!(
                "{slot_count}\t{present_slots}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                costs.sparse_w512_bits,
                costs.dense_w128_bits,
                costs.hybrid_w64_boolean_bits,
                costs.exact_minimum_mask(),
                u8::from(reduced.sparse_le_dense),
                u8::from(reduced.sparse_le_hybrid),
                u8::from(reduced.hybrid_le_dense),
            );
            rows = rows
                .checked_add(1)
                .ok_or_else(|| "row counter overflow".to_owned())?;
        }
    }

    let expected = expected_rows(max_slots)?;
    if rows != expected {
        return Err(format!(
            "vector row count drift: observed {rows}, expected {expected}"
        ));
    }
    eprintln!(
        "generated {rows} exact vectors under {VECTOR_SCHEMA} from {SLHA_CONTROL_PROFILE_SOURCE_REVISION}"
    );
    Ok(())
}

fn main() -> ExitCode {
    match parse_max_slots().and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("usage: bl_elastic_profile_vectors [max_slots]");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_row_formula_matches_small_domains() {
        assert_eq!(expected_rows(1).unwrap(), 2);
        assert_eq!(expected_rows(2).unwrap(), 5);
        assert_eq!(expected_rows(3).unwrap(), 9);
        assert_eq!(expected_rows(256).unwrap(), 33_152);
        assert_eq!(expected_rows(4096).unwrap(), 8_394_752);
    }

    #[test]
    fn parser_is_bounded_and_defaults_to_256() {
        assert_eq!(parse_max_slots_from(Vec::<String>::new()).unwrap(), 256);
        assert_eq!(parse_max_slots_from(["1".to_owned()]).unwrap(), 1);
        assert!(parse_max_slots_from(["0".to_owned()]).is_err());
        assert!(parse_max_slots_from(["4097".to_owned()]).is_err());
        assert!(parse_max_slots_from(["2".to_owned(), "3".to_owned()]).is_err());
    }

    #[test]
    fn representative_vector_replays_exact_oracle() {
        let costs = payloads(64, 1).unwrap();
        let reduced = reduced_predicates(64, 1).unwrap();
        assert_eq!(costs.exact_minimum_mask(), 1);
        assert!(reduced.sparse_le_dense);
        assert!(reduced.sparse_le_hybrid);
        assert!(reduced.hybrid_le_dense);
    }
}
