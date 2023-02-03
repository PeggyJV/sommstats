//! SommStats Config
//!
//! See instructions in `commands.rs` to specify the path to your
//! application's configuration file and/or command-line options
//! for specifying it.

use serde::{Deserialize, Serialize};

const HOUR_IN_SECS: u64 = 3600;
pub const DEFAULT_SNAPSHOT_FILE: &str = "sommstats_snapshot.json";

pub fn validate(config: &SommStatsConfig) {
    if config.grpc.endpoints.is_empty() {
        panic!("No gRPC endpoints specified in config");
    }
    if config.cache.community_pool_update_period == 0
        || config.cache.staking_update_period == 0
        || config.cache.foundation_wallet_update_period == 0
        || config.cache.vesting_update_period == 0
    {
        panic!("update periods must be greater than 0");
    }
}

/// SommStats Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SommStatsConfig {
    /// An example configuration section
    pub grpc: GrpcSection,
    pub server: ServerSection,
    pub cache: CacheSection,
}

/// Default configuration settings.
///
/// Note: if your needs are as simple as below, you can
/// use `#[derive(Default)]` on SommStatsConfig instead.
impl Default for SommStatsConfig {
    fn default() -> Self {
        Self {
            grpc: GrpcSection::default(),
            server: ServerSection::default(),
            cache: CacheSection::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GrpcSection {
    pub endpoints: Vec<String>,
    pub failed_query_retries: u32,
}

impl Default for GrpcSection {
    fn default() -> Self {
        GrpcSection {
            endpoints: Vec::new(),
            failed_query_retries: 3,
        }
    }
}

/// SommStats Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSection {
    pub address: String,
    pub port: u32,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            address: String::from("0.0.0.0"),
            port: 3000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CacheSection {
    pub community_pool_update_period: u64,
    pub staking_update_period: u64,
    pub vesting_update_period: u64,
    pub foundation_wallet_update_period: u64,
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            community_pool_update_period: HOUR_IN_SECS,
            staking_update_period: HOUR_IN_SECS,
            vesting_update_period: HOUR_IN_SECS,
            foundation_wallet_update_period: HOUR_IN_SECS,
        }
    }
}
