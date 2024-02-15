use std::{collections::HashMap, str::FromStr};

use crate::application::{
    ACTIVE_AUCTIONS, BIDS_BY_ACTIVE_AUCTION, ENDED_AUCTIONS, PRICE_BY_AUCTION,
};
use abscissa_core::tracing::{error, info};
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use sommelier_auction::{auction::Auction, client::Client, denom::Denom};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub current_unit_price_in_usomm: f64,
    pub token_decimals: u8,
    pub token_contract: String,
    pub token_symbol: String,
}

/// Updates the cached Active Auctions map
pub async fn update_active_auctions(endpoint: &str) -> Result<()> {
    let mut client = Client::with_endpoints("".to_string(), endpoint.to_string()).await?;

    match client.active_auctions().await {
        Ok(aa) => {
            let map: HashMap<u32, Auction> = aa.into_iter().map(|a| (a.id, a)).collect();
            let mut cache = ACTIVE_AUCTIONS.lock().await;

            *cache = map;

            info!("updated active auctions cache");

            Ok(())
        }
        Err(e) => {
            bail!(
                "error querying active auctions from endpoint {}: {:?}",
                endpoint,
                e
            );
        }
    }
}

/// Updates the cached Ended Auctions map
pub async fn update_ended_auctions(endpoint: &str) -> Result<()> {
    let mut client = Client::with_endpoints("".to_string(), endpoint.to_string()).await?;

    match client.ended_auctions().await {
        Ok(ea) => {
            let map: HashMap<u32, Auction> = ea.into_iter().map(|a| (a.id, a)).collect();
            let mut cache = ENDED_AUCTIONS.lock().await;

            *cache = map;

            info!("updated ended auctions cache");

            Ok(())
        }
        Err(e) => {
            bail!(
                "error querying ended auctions from endpoint {}: {:?}",
                endpoint,
                e
            );
        }
    }
}

/// Updates the cached Bids by Active Auction map
pub async fn update_bids_by_active_auction(endpoint: &str) -> Result<()> {
    let mut client = Client::with_endpoints("".to_string(), endpoint.to_string()).await?;

    let aa_cache = ACTIVE_AUCTIONS.lock().await;
    let mut bbaa_cache = BIDS_BY_ACTIVE_AUCTION.lock().await;

    for (id, _) in aa_cache.iter() {
        match client.auction_bids(*id).await {
            Ok(bids) => {
                bbaa_cache.insert(*id, bids);
            }
            Err(e) => {
                bail!(
                    "error querying bids for active auction {} from endpoint {}: {:?}",
                    id,
                    endpoint,
                    e
                );
            }
        }
    }

    info!("updated bids by active auction cache");

    Ok(())
}

/// Updates the price by active auction cache
pub async fn update_price_by_active_auction() -> Result<()> {
    let aa_cache = ACTIVE_AUCTIONS.lock().await;
    let mut pbaa_cache = PRICE_BY_AUCTION.lock().await;

    for (id, auction) in aa_cache.iter() {
        let price = match get_price(auction) {
            Ok(p) => p,
            Err(e) => {
                error!("error getting price for active auction {id}: {e:?}");
                continue;
            }
        };

        pbaa_cache.insert(*id, price);
    }

    info!("updated price by active auction cache");

    Ok(())
}

fn get_price(auction: &Auction) -> Result<Price> {
    let Some(coin) = auction.starting_tokens_for_sale.clone() else {
        bail!(
            "starting_tokens_for_sale is None for auction {}",
            auction.id
        )
    };
    let Ok(denom) = Denom::from_str(&coin.denom) else {
        bail!(
            "denom {} is not a valid sommelier_auction::Denom",
            coin.denom
        )
    };
    let Some(contract_address) = coin.denom.strip_prefix("gravity") else {
        bail!("denom {} does not start with 'gravity'", coin.denom)
    };
    // conversion for sdk.Dec which is a BigInt underneath
    let unit_price_in_usomm_int = auction.current_unit_price_in_usomm.parse::<f64>()?;
    let unit_price_in_usomm = unit_price_in_usomm_int / 10f64.powi(18_i32);

    Ok(Price {
        current_unit_price_in_usomm: unit_price_in_usomm,
        token_decimals: denom.decimals(),
        token_contract: contract_address.to_string(),
        token_symbol: denom.symbol(),
    })
}
