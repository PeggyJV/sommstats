//! SommelierApi Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};

/// SommelierApi Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SommelierApiConfig {
    /// An example configuration section
    pub grpc: String,
}

/// Default configuration settings.
///
/// Note: if your needs are as simple as below, you can
/// use `#[derive(Default)]` on SommelierApiConfig instead.
impl Default for SommelierApiConfig {
    fn default() -> Self {
        Self {
            grpc: String::default(),
        }
    }
}
