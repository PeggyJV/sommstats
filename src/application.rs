//! SommStats Abscissa Application

use std::{collections::HashMap, sync::Arc};

use crate::{auction::Price, commands::EntryPoint, config::SommStatsConfig};
use abscissa_core::{
    application::{self, AppCell},
    config::{self, CfgCell},
    trace, Application, FrameworkError, StandardPaths,
};
use abscissa_tokio::tokio::sync::Mutex;
use lazy_static::lazy_static;
use sommelier_auction::auction::{Auction, Bid};

pub type Cache<T> = Arc<Mutex<T>>;

pub const USOMM: &str = "usomm";

lazy_static! {
    pub static ref ACTIVE_AUCTIONS: Cache<HashMap<u32, Auction>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref ENDED_AUCTIONS: Cache<HashMap<u32, Auction>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref BIDS_BY_ACTIVE_AUCTION: Cache<HashMap<u32, Vec<Bid>>> = Arc::new(Mutex::new(HashMap::new()));
    pub static ref PRICE_BY_AUCTION: Cache<HashMap<u32, Price>> = Arc::new(Mutex::new(HashMap::new()));

    /// Balances cache, where each key is the ID of the balance, either an address in the case of
    /// vesting accounts, or a designation such as "communitypool" or "bonded" in the case of
    /// the community pool and total bonded token balances. Addresses that are not the foundation
    /// address can be safely assumed to be vesting addresses.
    pub static ref BALANCES: Cache<HashMap<String, u64>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Application state
pub static APP: AppCell<SommStatsApp> = AppCell::new();

/// SommStats Application
#[derive(Debug)]
pub struct SommStatsApp {
    /// Application configuration.
    config: CfgCell<SommStatsConfig>,

    /// Application state.
    state: application::State<Self>,
}

/// Initialize a new application instance.
///
/// By default no configuration is loaded, and the framework state is
/// initialized to a default, empty state (no components, threads, etc).
impl Default for SommStatsApp {
    fn default() -> Self {
        Self {
            config: CfgCell::default(),
            state: application::State::default(),
        }
    }
}

impl Application for SommStatsApp {
    /// Entrypoint command for this application.
    type Cmd = EntryPoint;

    /// Application configuration.
    type Cfg = SommStatsConfig;

    /// Paths to resources within the application.
    type Paths = StandardPaths;

    /// Accessor for application configuration.
    fn config(&self) -> config::Reader<SommStatsConfig> {
        self.config.read()
    }

    /// Borrow the application state immutably.
    fn state(&self) -> &application::State<Self> {
        &self.state
    }

    /// Register all components used by this application.
    ///
    /// If you would like to add additional components to your application
    /// beyond the default ones provided by the framework, this is the place
    /// to do so.
    fn register_components(&mut self, command: &Self::Cmd) -> Result<(), FrameworkError> {
        let mut framework_components = self.framework_components(command)?;
        let mut app_components = self.state.components_mut();
        framework_components.push(Box::new(abscissa_tokio::TokioComponent::new()?));
        app_components.register(framework_components)
    }

    /// Post-configuration lifecycle callback.
    ///
    /// Called regardless of whether config is loaded to indicate this is the
    /// time in app lifecycle when configuration would be loaded if
    /// possible.
    fn after_config(&mut self, config: Self::Cfg) -> Result<(), FrameworkError> {
        // Configure components
        let mut components = self.state.components_mut();
        components.after_config(&config)?;
        self.config.set_once(config);
        Ok(())
    }

    /// Get tracing configuration from command-line options
    fn tracing_config(&self, command: &EntryPoint) -> trace::Config {
        if command.verbose {
            trace::Config::verbose()
        } else {
            match std::env::var("RUST_LOG") {
                Ok(val) => {
                    if !val.is_empty() {
                        val.into()
                    } else {
                        trace::Config::default()
                    }
                }
                Err(_) => trace::Config::default(),
            }
        }
    }
}
