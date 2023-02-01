//! Main entry point for SommStats

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use sommstats::application::APP;

/// Boot SommStats
fn main() {
    abscissa_core::boot(&APP);
}
