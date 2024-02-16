use std::str::FromStr;

use abscissa_core::{
    tracing::{debug, error},
    Application,
};
use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use sommelier_auction::{
    auction::{Auction as AuctionProto, Bid as BidProto},
    denom::Denom,
};

use crate::{
    application::{ACTIVE_AUCTIONS, BIDS_BY_ACTIVE_AUCTION, ENDED_AUCTIONS},
    auction::cache::update_bids_by_active_auction,
    prelude::APP,
    server::json_response,
    utils,
};

use self::cache::{update_active_auctions, update_ended_auctions};

pub mod cache;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auction {
    pub id: u32,
    pub start_block: u64,
    pub end_block: u64,
    pub cellar_fee_token_for_sale: CellarFeeToken,
    pub initial_supply: u128,
    pub remaining_supply: u128,
    pub initial_unit_price_in_usomm: f64,
    pub current_unit_price_in_usomm: f64,
    pub initial_price_decrease_rate: f64,
    pub current_price_decrease_rate: f64,
    pub price_decrease_block_interval: u64,
}

impl TryFrom<AuctionProto> for Auction {
    type Error = eyre::Report;

    fn try_from(auction: AuctionProto) -> Result<Self> {
        let Some(starting_coin) = auction.starting_tokens_for_sale.clone() else {
            bail!(
                "starting_tokens_for_sale is None for auction {}",
                auction.id
            )
        };
        let Some(remaining_coin) = auction.remaining_tokens_for_sale.clone() else {
            bail!(
                "remaining_tokens_for_sale is None for auction {}",
                auction.id
            )
        };
        let initial_supply = starting_coin.amount.parse::<u128>()?;
        let remaining_supply = remaining_coin.amount.parse::<u128>()?;
        let cellar_fee_token_for_sale = CellarFeeToken::try_from(starting_coin.denom)?;
        let initial_unit_price_in_usomm =
            utils::sdk_dec_string_to_f64(auction.initial_unit_price_in_usomm)?;
        let current_unit_price_in_usomm =
            utils::sdk_dec_string_to_f64(auction.current_unit_price_in_usomm)?;
        let initial_price_decrease_rate =
            utils::sdk_dec_string_to_f64(auction.initial_price_decrease_rate)?;
        let current_price_decrease_rate =
            utils::sdk_dec_string_to_f64(auction.current_price_decrease_rate)?;

        Ok(Auction {
            id: auction.id,
            start_block: auction.start_block,
            end_block: auction.end_block,
            cellar_fee_token_for_sale,
            initial_supply,
            remaining_supply,
            initial_unit_price_in_usomm,
            current_unit_price_in_usomm,
            initial_price_decrease_rate,
            current_price_decrease_rate,
            price_decrease_block_interval: auction.price_decrease_block_interval,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub id: u64,
    pub auction_id: u32,
    pub cellar_fee_token: CellarFeeToken,
    pub bidder: String,
    pub max_bid_in_usomm: u128,
    pub sale_token_minimum_amount: u128,
    pub total_usomm_paid: u128,
    pub total_fulfilled_sale_tokens: u128,
    pub sale_token_unit_price_in_usomm: f64,
    pub block_height: u64,
}

impl TryFrom<BidProto> for Bid {
    type Error = eyre::Report;

    fn try_from(bid: BidProto) -> Result<Self> {
        let Some(max_bid) = bid.max_bid_in_usomm.clone() else {
            bail!("max_bid_in_usomm is None for auction {}", bid.id)
        };
        let Some(min_out) = bid.sale_token_minimum_amount.clone() else {
            bail!("sale_token_minimum_amount is None for auction {}", bid.id)
        };
        let Some(total_usomm_paid) = bid.total_usomm_paid.clone() else {
            bail!("total_usomm_paid is None for auction {}", bid.id)
        };
        let Some(total_fulfilled_sale_tokens) = bid.total_fulfilled_sale_tokens.clone() else {
            bail!("total_fulfilled_sale_tokens is None for auction {}", bid.id)
        };
        let max_bid_in_usomm = max_bid.amount.parse::<u128>()?;
        let sale_token_minimum_amount = min_out.amount.parse::<u128>()?;
        let total_usomm_paid = total_usomm_paid.amount.parse::<u128>()?;
        let total_fulfilled_sale_tokens = total_fulfilled_sale_tokens.amount.parse::<u128>()?;
        let cellar_fee_token = CellarFeeToken::try_from(min_out.denom)?;
        let sale_token_unit_price_in_usomm =
            utils::sdk_dec_string_to_f64(bid.sale_token_unit_price_in_usomm)?;

        Ok(Bid {
            id: bid.id,
            auction_id: bid.auction_id,
            cellar_fee_token,
            sale_token_minimum_amount,
            bidder: bid.bidder,
            max_bid_in_usomm,
            total_usomm_paid,
            total_fulfilled_sale_tokens,
            sale_token_unit_price_in_usomm,
            block_height: bid.block_height,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellarFeeToken {
    pub symbol: String,
    pub sommelier_denom: String,
    pub decimals: u8,
    pub origin_chain_id: u32,
    pub contract_address: String,
}

impl TryFrom<String> for CellarFeeToken {
    type Error = eyre::Report;

    fn try_from(value: String) -> Result<Self> {
        let Ok(denom) = Denom::from_str(&value) else {
            bail!("value {} is not a valid sommelier_auction::Denom", value)
        };
        let Some(contract_address) = value.strip_prefix("gravity") else {
            bail!("denom {} does not start with 'gravity'", value)
        };

        Ok(CellarFeeToken {
            contract_address: contract_address.to_string(),
            symbol: denom.symbol(),
            decimals: denom.decimals(),
            sommelier_denom: denom.to_string(),
            ..Default::default()
        })
    }
}

impl Default for CellarFeeToken {
    fn default() -> Self {
        CellarFeeToken {
            contract_address: "".to_string(),
            symbol: "".to_string(),
            decimals: 0,
            sommelier_denom: "".to_string(),
            // default to Ethereum until x/cellarfees supports Axelar-originated fees
            origin_chain_id: 1,
        }
    }
}

// Response types and handlers

//.route("/v1/auctions/active", get(get_active_auctions))
//        .route("/v1/auctions/ended", get(get_ended_auction))
//        .route("/v1/auctions/:id", get(get_auction_by_id))
//        .route("/v1/auctions/:id/bids", get(get_bids_by_auction_id))

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionResponse {
    pub auction: Option<Auction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionsResponse {
    pub auctions: Vec<Auction>,
}

/// Handler for `GET /v1/auctions/active`
pub async fn get_active_auctions() -> axum::response::Response {
    debug!("GET /v1/auctions/active");
    if ACTIVE_AUCTIONS.read().await.is_expired() {
        let config = APP.config();
        let Some(endpoint) = config.grpc.endpoints.get(0) else {
            error!("no gRPC endpoints configured");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        // log and return a response anyway
        if let Err(err) = update_active_auctions(endpoint).await {
            error!("failed to update active auctions: {err:?}");
        }
    }

    let cache = ACTIVE_AUCTIONS.read().await;
    let response = AuctionsResponse {
        auctions: cache.data.values().cloned().collect::<Vec<Auction>>(),
    };

    json_response(response)
}

/// Handler for `GET /v1/auctions/ended`
pub async fn get_ended_auction() -> axum::response::Response {
    debug!("GET /v1/auctions/ended");
    if ENDED_AUCTIONS.read().await.is_expired() {
        let config = APP.config();
        let Some(endpoint) = config.grpc.endpoints.get(0) else {
            error!("no gRPC endpoints configured");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        // log and return a response anyway
        if let Err(err) = update_ended_auctions(endpoint).await {
            error!("failed to update ended auctions: {err:?}");
        }
    }

    let cache = ENDED_AUCTIONS.read().await;
    let response = AuctionsResponse {
        auctions: cache.data.values().cloned().collect::<Vec<Auction>>(),
    };

    json_response(response)
}

/// Handler for `GET /v1/auctions/:id`
pub async fn get_auction_by_id(Path(id): Path<u32>) -> axum::response::Response {
    debug!("GET /v1/auctions/{id}");
    let config = APP.config();
    let Some(endpoint) = config.grpc.endpoints.get(0) else {
        error!("no gRPC endpoints configured");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let aa_cache = ACTIVE_AUCTIONS.read().await;
    // we don't do this check and update active auctions because usually the cache will be empty
    // and it would be spammable
    if ENDED_AUCTIONS.read().await.data.is_empty() {
        if let Err(err) = update_ended_auctions(&endpoint).await {
            error!("failed to update ended auctions: {err:?}");
        }
    }
    let ea_cache = ENDED_AUCTIONS.read().await;

    let mut is_active = false;
    let mut is_ended = false;

    if aa_cache.data.get(&id).is_some() {
        is_active = true;
    }
    if ea_cache.data.get(&id).is_some() {
        is_ended = true;
    }

    let mut auction = None;
    if is_active {
        auction = aa_cache.data.get(&id).cloned();
    } else if is_ended {
        auction = ea_cache.data.get(&id).cloned();
    }

    let response = AuctionResponse { auction };

    json_response(response)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidsByAuctionResponse {
    pub bids: Vec<Bid>,
}

/// Handler for `GET /v1/auctions/:id/bids`
pub async fn get_bids_by_auction_id(Path(id): Path<u32>) -> axum::response::Response {
    debug!("GET /v1/auctions/{id}/bids");

    if ACTIVE_AUCTIONS.read().await.is_expired() {
        let config = APP.config();
        let Some(endpoint) = config.grpc.endpoints.get(0) else {
            error!("no gRPC endpoints configured");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        if let Err(err) = update_active_auctions(&endpoint).await {
            error!("failed to update active auctions: {err:?}");
        }
    }
    if BIDS_BY_ACTIVE_AUCTION.read().await.is_expired() {
        let config = APP.config();
        let Some(endpoint) = config.grpc.endpoints.get(0) else {
            error!("no gRPC endpoints configured");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };

        if let Err(err) = update_bids_by_active_auction(&endpoint).await {
            error!("failed to update bids: {err:?}");
        }
    }

    let cache = BIDS_BY_ACTIVE_AUCTION.read().await;
    let bids = cache.data.get(&id).cloned().unwrap_or_default();

    if bids.is_empty() {
        return json_response(BidsByAuctionResponse { bids: Vec::new() });
    }

    let bids = match bids
        .into_iter()
        .map(|b| Bid::try_from(b))
        .collect::<Result<Vec<Bid>, _>>()
    {
        Ok(bids) => bids,
        Err(err) => {
            error!("failed to convert bids: {err:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let response = BidsByAuctionResponse { bids };

    json_response(response)
}
