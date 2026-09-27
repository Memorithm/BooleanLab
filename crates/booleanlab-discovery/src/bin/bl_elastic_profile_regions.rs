#![forbid(unsafe_code)]

//! Deterministic export of compact BL-BE5 Elastic control-profile regions.

use booleanlab_discovery::elastic_control_profile_partition::SLHA_CONTROL_PROFILE_SOURCE_REVISION;
use booleanlab_discovery::elastic_control_profile_regions::{
    ELASTIC_CONTROL_PROFILE_REGIONS_V1, regions_for_slot_count,
};
use std::env;
use std::process::ExitCode;

const EXPORT_SCHEMA: &str = "booleanlab.elastic-control-profile-region-export@1.0.0";
const DEFAULT_MAX_SLOTS: usize = 256;
const QUALIFIED_MAX_SLOTS: usize = 4096;

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
    if max_slots == 0 || max_slots > QUALIFIED_MAX_SLOTS {
        return Err(format!(
            "max_slots must be in 1..={QUALIFIED_MAX_SLOTS}, observed {max_slots}"
        ));
    }
    Ok(max_slots)
}

fn parse_max_slots() -> Result<usize, String> {
    parse_max_slots_from(env::args().skip(1))
}

fn run(max_slots: usize) -> Result<(), String> {
    println!("# schema={EXPORT_SCHEMA}");
    println!("# region_contract={ELASTIC_CONTROL_PROFILE_REGIONS_V1}");
    println!("# source_revision={SLHA_CONTROL_PROFILE_SOURCE_REVISION}");
    println!("# max_slots={max_slots}");
    println!("slot_count\tpresent_min\tpresent_max\tminimum_mask");

    let mut rows = 0_u64;
    for slot_count in 1..=max_slots {
        for region in regions_for_slot_count(slot_count).map_err(|error| error.to_string())? {
            println!(
                "{slot_count}\t{}\t{}\t{}",
                region.present_min(),
                region.present_max(),
                region.minimum_mask()
            );
            rows = rows
                .checked_add(1)
                .ok_or_else(|| "region row counter overflow".to_owned())?;
        }
    }

    eprintln!(
        "generated {rows} compact exact regions for slot_count=1..={max_slots} under {EXPORT_SCHEMA}"
    );
    Ok(())
}

fn main() -> ExitCode {
    match parse_max_slots().and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("usage: bl_elastic_profile_regions [max_slots]");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booleanlab_discovery::elastic_control_profile_partition::payloads;
    use booleanlab_discovery::elastic_control_profile_regions::minimum_mask_from_regions;

    #[test]
    fn parser_defaults_and_bounds_are_exact() {
        assert_eq!(parse_max_slots_from(Vec::<String>::new()).unwrap(), 256);
        assert_eq!(parse_max_slots_from(["1".to_owned()]).unwrap(), 1);
        assert_eq!(
            parse_max_slots_from([QUALIFIED_MAX_SLOTS.to_string()]).unwrap(),
            QUALIFIED_MAX_SLOTS
        );
        assert!(parse_max_slots_from(["0".to_owned()]).is_err());
        assert!(parse_max_slots_from([(QUALIFIED_MAX_SLOTS + 1).to_string()]).is_err());
        assert!(parse_max_slots_from(["2".to_owned(), "3".to_owned()]).is_err());
    }

    #[test]
    fn representative_export_regions_replay_exact_oracle() {
        for slot_count in [1, 3, 16, 64, 256] {
            for present_slots in 0..=slot_count {
                let expected = payloads(slot_count, present_slots)
                    .unwrap()
                    .exact_minimum_mask();
                let compact = minimum_mask_from_regions(slot_count, present_slots).unwrap();
                assert_eq!(compact, expected);
            }
        }
    }

    #[test]
    fn region_export_is_smaller_than_full_vector_export_for_default_domain() {
        let region_rows = (1..=DEFAULT_MAX_SLOTS)
            .map(|slot_count| regions_for_slot_count(slot_count).unwrap().len())
            .sum::<usize>();
        let full_rows = (1..=DEFAULT_MAX_SLOTS)
            .map(|slot_count| slot_count + 1)
            .sum::<usize>();

        assert!(region_rows < full_rows);
    }
}
