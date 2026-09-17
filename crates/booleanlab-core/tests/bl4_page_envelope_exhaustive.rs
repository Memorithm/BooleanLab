#![forbid(unsafe_code)]

use booleanlab_core::{
    BitSignature, PageEnvelope, admit_page_by_hamming_lower_bound, hamming_distance,
    page_hamming_lower_bound,
};

const BITS: usize = 4;
const VALUES: u8 = 1 << BITS;

fn signature(value: u8) -> BitSignature {
    let bits = (0..BITS)
        .map(|bit| value & (1 << bit) != 0)
        .collect::<Vec<_>>();
    BitSignature::from_bits(&bits).expect("bounded four-bit signature")
}

#[test]
fn exhaustive_four_bit_two_key_pages_never_false_reject() {
    let mut checked = 0usize;

    for query_value in 0..VALUES {
        let query = signature(query_value);

        // Enumerate every unordered two-key multiset, including equal keys.
        // Equal-key pairs cover the fully unanimous envelope boundary while
        // distinct pairs cover all possible variable-bit combinations.
        for left_value in 0..VALUES {
            for right_value in left_value..VALUES {
                let keys = [signature(left_value), signature(right_value)];
                let page = PageEnvelope::from_keys(&keys).expect("valid equal-width page");
                let lower_bound =
                    page_hamming_lower_bound(&query, &page).expect("matching signature widths");
                let exact_distances = keys
                    .iter()
                    .map(|key| hamming_distance(&query, key).expect("matching signature widths"))
                    .collect::<Vec<_>>();
                let exact_min = *exact_distances.iter().min().expect("two-key page");

                // The page envelope is permitted to be loose, never optimistic.
                assert!(
                    lower_bound <= exact_min,
                    "query={query_value:04b} keys=[{left_value:04b},{right_value:04b}] lower_bound={lower_bound} exact_min={exact_min}"
                );

                for max_distance in 0..=BITS as u64 {
                    let page_admitted = admit_page_by_hamming_lower_bound(
                        &query,
                        &page,
                        max_distance,
                    )
                    .expect("bounded threshold");
                    let exact_any_admitted = exact_distances
                        .iter()
                        .any(|&distance| distance <= max_distance);

                    // This is the BL-4.5.1 safety invariant: page-level early
                    // rejection must never discard a key that the exact pair
                    // Hamming rule would have retained.
                    assert!(
                        page_admitted || !exact_any_admitted,
                        "false-negative page rejection: query={query_value:04b} keys=[{left_value:04b},{right_value:04b}] threshold={max_distance} distances={exact_distances:?}"
                    );
                    checked += 1;
                }
            }
        }
    }

    // 16 queries × C(16 + 2 - 1, 2) unordered two-key multisets × 5 thresholds.
    assert_eq!(checked, 16 * 136 * 5);
}
