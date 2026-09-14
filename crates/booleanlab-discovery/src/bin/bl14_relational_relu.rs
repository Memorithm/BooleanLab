//! BL-14.4.1 relation-aware sparsity development runner.
//!
//! The numerical workload is included from the existing BL-14.2.2 runner so
//! this binary cannot silently drift to a second model implementation.

mod base {
    #![allow(dead_code)]

    include!("bl14_structured_relu.rs");

    pub(crate) mod relational {
        include!("support/bl14_relational.rs");
    }

    pub(crate) fn run_relational_entry() -> std::result::Result<(), Box<dyn std::error::Error>> {
        relational::run()
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    base::run_relational_entry()
}
