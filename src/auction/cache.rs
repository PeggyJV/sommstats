use std::collections::HashMap;
use std::time::Duration;

use crate::{
    application::{ACTIVE_AUCTIONS, BIDS_BY_ACTIVE_AUCTION, ENDED_AUCTIONS},
    auction::Auction,
    prelude::APP,
};
use abscissa_core::{
    tracing::{debug, info},
    Application,
};
use eyre::{bail, Result};
use sommelier_auction::client::Client;

/// Updates the cached Active Auctions map
pub async fn update_active_auctions(endpoint: &str) -> Result<()> {
    info!("updating active auctions cache");
    let mut client = Client::with_endpoints("".to_string(), endpoint.to_string()).await?;
    debug!("active auctions client created");

    match client.active_auctions().await {
        Ok(aa) => {
            let auctions = aa
                .into_iter()
                .map(|a| Auction::try_from(a))
                .collect::<Result<Vec<Auction>>>()?;
            let map: HashMap<u32, Auction> = auctions.into_iter().map(|a| (a.id, a)).collect();

            debug!("getting active auctions cache lock");
            let mut cache = ACTIVE_AUCTIONS.write().await;

            cache.data = map;

            info!("updated active auctions cache");

            let config = APP.config();
            cache.set_expiration(Duration::from_secs(
                config.cache.active_auctions_update_period,
            ));

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
            let auctions = ea
                .into_iter()
                .map(|a| Auction::try_from(a))
                .collect::<Result<Vec<Auction>>>()?;
            let map: HashMap<u32, Auction> = auctions.into_iter().map(|a| (a.id, a)).collect();
            let mut cache = ENDED_AUCTIONS.write().await;

            cache.data = map;

            info!("updated ended auctions cache");

            let config = APP.config();
            cache.set_expiration(Duration::from_secs(
                config.cache.active_auctions_update_period,
            ));

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

    let aa_cache = ACTIVE_AUCTIONS.read().await;
    let mut bbaa_cache = BIDS_BY_ACTIVE_AUCTION.write().await;

    for (id, _) in aa_cache.data.iter() {
        match client.auction_bids(*id).await {
            Ok(bids) => {
                bbaa_cache.data.insert(*id, bids);
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

    let config = APP.config();
    bbaa_cache.set_expiration(Duration::from_secs(
        config.cache.active_auctions_update_period,
    ));

    Ok(())
}
