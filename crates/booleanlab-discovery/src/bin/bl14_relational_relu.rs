//! BL-14.4.1 relation-aware sparsity development runner.
//!
//! The numerical workload is loaded as the existing BL-14.2.2 module so this
//! binary cannot silently drift to a second model implementation.

#[path = "bl14_structured_relu.rs"]
#[allow(dead_code)]
mod base;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    base::run_relational_entry()
}
