use std::net::SocketAddr;

use abscissa_core::tracing::{
    info,
    log::{error, warn},
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{
    accounting::{FOUNDATION_ADDRESS, TOTAL_USOMM_SUPPLY, VESTING_ACCOUNTS},
    application::BALANCES,
    query::{COMMUNITY_POOL_KEY, STAKING_BALANCE_KEY},
};

pub async fn listen(addr: SocketAddr) -> Result<()> {
    let app = Router::new().route("/api/v1/circulating-supply", get(get_circulating_supply));

    info!("listening on {}", addr);
    Ok(axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CirculatingSupplyResponse {
    pub circulating_supply: u128,
}

/// Calculates and returns the circulating supply. If one or more balance is not populated in the cache,
/// returns a 503 status code.
/// Circulating supply == Total supply - Foundation wallet - Staking - Community Pool - Vesting balances
pub async fn get_circulating_supply() -> Response {
    let balances = BALANCES.lock().await;
    // instead of just summing all entries we get them individually to make sure none are missing,
    // which would make our calculation overshoot the actual circulating supply.
    let mut less = vec![
        (FOUNDATION_ADDRESS, balances.get(FOUNDATION_ADDRESS)),
        (STAKING_BALANCE_KEY, balances.get(STAKING_BALANCE_KEY)),
        (COMMUNITY_POOL_KEY, balances.get(COMMUNITY_POOL_KEY)),
    ];
    VESTING_ACCOUNTS
        .iter()
        .for_each(|v| less.push((v, balances.get(*v))));

    if let Some(unpopulated) = less.iter().find(|v| v.1.is_none()) {
        warn!(
            "circulating supply request failed due to missing balance for {}",
            unpopulated.0
        );
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let circulating_supply = TOTAL_USOMM_SUPPLY - less.iter().map(|v| v.1.unwrap()).sum::<u128>();
    let body = serde_json::to_string(&CirculatingSupplyResponse { circulating_supply });
    if let Err(e) = body {
        error!("error serializing circulating supply response: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    json_response(body.unwrap())
}

pub fn json_response(body: String) -> Response {
    Response::builder()
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| {
            error!("error building response: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;
    use axum::http::StatusCode;

    #[assay]
    async fn get_circulating_supply_service_unavailable() {
        let expected = StatusCode::SERVICE_UNAVAILABLE;
        let actual = get_circulating_supply().await;

        assert_eq!(expected, actual.status());
    }
}
