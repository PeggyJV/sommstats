//! `start` subcommand - example of how to write a subcommand

use std::net::SocketAddr;

use crate::config::SommStatsConfig;
/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;
use crate::query::poll_vesting_balance;
use crate::query::{poll_community_pool_balance, poll_foundation_balance, poll_staking_balance};
use crate::server::listen;

use abscissa_core::config::Override;
use abscissa_core::{Command, FrameworkError, Runnable};
use abscissa_tokio::tokio::join;
use clap::Parser;

/// `start` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct StartCmd {}

impl Runnable for StartCmd {
    /// Start the application.
    fn run(&self) {
        let config = APP.config();
        abscissa_tokio::run(&APP, async {
            let addr: SocketAddr = format!("{}:{}", config.server.address, config.server.port)
                .parse()
                .expect("failed to parse socket address");
            let _ = join!(
                poll_vesting_balance(),
                poll_foundation_balance(),
                poll_community_pool_balance(),
                poll_staking_balance(),
                listen(addr)
            );
        })
        .unwrap_or_else(|e| {
            status_err!("executor exited with error: {}", e);
            std::process::exit(1)
        });
    }
}

impl Override<SommStatsConfig> for StartCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(
        &self,
        config: SommStatsConfig,
    ) -> Result<SommStatsConfig, FrameworkError> {
        Ok(config)
    }
}
