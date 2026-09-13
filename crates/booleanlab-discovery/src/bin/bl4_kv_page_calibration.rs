use booleanlab_core::{
    BitSignature, PageEnvelope, admit_page_by_hamming_lower_bound, hamming_distance,
    page_hamming_lower_bound,
};

fn signature(bits: &[bool]) -> Result<BitSignature, Box<dyn std::error::Error>> {
    Ok(BitSignature::from_bits(bits)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = signature(&[true, false, true, false, true, false, true, false])?;
    let pages = [
        vec![
            signature(&[true, false, true, false, true, false, true, false])?,
            signature(&[false, false, true, false, true, false, true, false])?,
        ],
        vec![
            signature(&[false, true, true, false, true, false, true, false])?,
            signature(&[false, true, false, false, true, false, true, false])?,
        ],
        vec![
            signature(&[false, true, false, false, true, false, true, false])?,
            signature(&[false, true, false, true, true, false, true, false])?,
        ],
        vec![signature(&[
            false, true, false, true, false, true, false, true,
        ])?],
    ];
    let threshold = 2_u64;

    println!("BL-4.5.1 deterministic Boolean KV-page calibration");
    println!("page\tkeys\tlower_bound\tmin_exact_distance\tadmitted");

    let mut decisions = Vec::with_capacity(pages.len());
    for (page_index, page_keys) in pages.iter().enumerate() {
        let envelope = PageEnvelope::from_keys(page_keys)?;
        let lower_bound = page_hamming_lower_bound(&query, &envelope)?;
        let admitted = admit_page_by_hamming_lower_bound(&query, &envelope, threshold)?;
        let min_exact_distance = page_keys
            .iter()
            .map(|key| hamming_distance(&query, key))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .expect("calibration pages are non-empty");

        assert!(lower_bound <= min_exact_distance);
        if !admitted {
            assert!(min_exact_distance > threshold);
        }

        decisions.push(admitted);
        println!(
            "{page_index}\t{}\t{lower_bound}\t{min_exact_distance}\t{admitted}",
            page_keys.len()
        );
    }

    assert_eq!(decisions, vec![true, true, false, false]);
    Ok(())
}
