//! `start` subcommand - example of how to write a subcommand

use crate::application::BALANCES;
/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;
use crate::query::track_vesting_balances;

use crate::config::SommelierApiConfig;
use abscissa_core::{config, Command, FrameworkError, Runnable};
use clap::Parser;
use ocular::QueryClient;

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
        let mut somm_qclient = QueryClient::new(&config.grpc.clone()).unwrap();
        // let mut osmo_qclient = QueryClient::new("https://osmosis-grpc.polkachu.com:12590").unwrap();

        abscissa_tokio::run(&APP, async {
            track_vesting_balances(&mut somm_qclient).await.unwrap();

            println!("total: {:?}", BALANCES.lock().await);

            // let somm_addresses = &SOMM_ADDRESSES.lock().await;
            // for address in somm_addresses.iter() {
            //     get_and_cache_balances(&mut somm_qclient, address)
            //         .await
            //         .unwrap();
            // }

            // let osmo_addresses = &OSMO_ADDRESSES.lock().await;
            // println!("OSMO ADDRESSES: {:?}", osmo_addresses);
            // for address in osmo_addresses.iter() {
            //     get_and_cache_balances(&mut osmo_qclient, address)
            //         .await
            //         .unwrap();
            // }

            // println!("{:?}", BALANCES.lock().await);
        })
        .unwrap_or_else(|e| {
            status_err!("executor exited with error: {}", e);
            std::process::exit(1)
        });
    }
}
