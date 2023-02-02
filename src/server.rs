use std::net::SocketAddr;

use abscissa_core::tracing::{info, log::error};
use axum::{http::StatusCode, routing::get, Router, response::{Response, IntoResponse}};
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{
    accounting::{FOUNDATION_ADDRESS, TOTAL_USOMM_SUPPLY},
    application::BALANCES,
    query::{COMMUNITY_POOL_KEY, STAKING_BALANCE_KEY, VESTING_BALANCE_KEY},
};

pub async fn listen(addr: SocketAddr) -> Result<()> {
    let app = Router::new().route("/circulating-supply", get(get_circulating_supply));

    info!("listening on {}", addr);
    Ok(axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?)
}

#[derive(Deserialize, Serialize)]
pub struct CirculatingSupplyResponse {
    pub circulating_supply: u128,
}

/// Calculates and returns the circulating supply. If one or more balance is not populated in the cache,
/// returns a 503 status code.
/// Circulating supply == Total supply - Foundation wallet - Staking - Community Pool - Vesting balances
pub async fn get_circulating_supply() -> Result<Response, StatusCode> {
    let balances = BALANCES.lock().await;
    let less = vec![
        balances.get(FOUNDATION_ADDRESS),
        balances.get(STAKING_BALANCE_KEY),
        balances.get(COMMUNITY_POOL_KEY),
        balances.get(VESTING_BALANCE_KEY),
    ];

    if less.iter().any(|v| v.is_none()) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let circulating_supply = TOTAL_USOMM_SUPPLY - less.iter().map(|v| v.unwrap()).sum::<u128>();
    let body = serde_json::to_string(&CirculatingSupplyResponse {
        circulating_supply
    }).map_err(|e| {
        error!("error serializing circulating supply response: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    json_response(body)
}

pub fn json_response(body: String) -> Result<Response, StatusCode> {
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| {
            error!("error building response: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
        .into_response())
}
