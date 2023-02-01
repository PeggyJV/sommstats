//! SommStats Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};

/// SommStats Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SommStatsConfig {
    /// An example configuration section
    pub grpc: String,
}

/// Default configuration settings.
///
/// Note: if your needs are as simple as below, you can
/// use `#[derive(Default)]` on SommStatsConfig instead.
impl Default for SommStatsConfig {
    fn default() -> Self {
        Self {
            grpc: String::default(),
        }
    }
}
