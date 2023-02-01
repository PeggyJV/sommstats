//! `start` subcommand - example of how to write a subcommand

use std::net::SocketAddr;

use crate::query::{update_foundation_balance, update_community_pool_balance, update_staking_balance, get_circulating_supply};
use crate::snapshot::{take_cache_snapshot, try_load_snapshot};
use crate::{query::update_vesting_balance};
/// App-local prelude includes `app_reader()`/`app_writer()`/`app_config()`
/// accessors along with logging macros. Customize as you see fit.
use crate::prelude::*;


use abscissa_core::{Command, Runnable};
use axum::Router;
use axum::routing::get;
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

        abscissa_tokio::run(&APP, async {
            if !try_load_snapshot().await.expect("failed to read snapshot file") {
                info!("no snapshot found. updating cache!");
                info!("updating vesting balances...");
                update_vesting_balance(&mut somm_qclient).await.unwrap();
                info!("updating foundation wallet balance...");
                update_foundation_balance(&mut somm_qclient).await.unwrap();
                info!("updating community pool balance...");
                update_community_pool_balance(&mut somm_qclient).await.unwrap();
                info!("updating staking balance...");
                update_staking_balance(&mut somm_qclient).await.unwrap();
                info!("taking a snapshot...");
                take_cache_snapshot().await.unwrap();
            }

            let app = Router::new()
                .route("/circulating-supply", get(get_circulating_supply));

            let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
            info!("listening on {}", addr);
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        })
        .unwrap_or_else(|e| {
            status_err!("executor exited with error: {}", e);
            std::process::exit(1)
        });
    }
}
