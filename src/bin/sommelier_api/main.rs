//! Main entry point for SommelierApi

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use sommelier_api::application::APP;

/// Boot SommelierApi
fn main() {
    abscissa_core::boot(&APP);
}
