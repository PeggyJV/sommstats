use abscissa_core::{
    tracing::{
        debug,
        log::{error, warn},
    },
    Application,
};
use abscissa_tokio::tokio;
use eyre::{bail, Result};
use ocular::{
    cosmrs::proto::{
        cosmos::{
            base::v1beta1::{Coin, DecCoin},
            vesting::v1beta1::{
                BaseVestingAccount, ContinuousVestingAccount, DelayedVestingAccount,
                PeriodicVestingAccount,
            },
        },
        traits::Message,
    },
    QueryClient,
};
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    Retry,
};

use crate::{
    accounting::{FOUNDATION_ADDRESS, VESTING_ACCOUNTS},
    application::{BALANCES, USOMM},
    prelude::APP,
};

const BASE_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.BaseVestingAccount";
const CONTINUOUS_VESTING_ACCOUNT_TYPE_URL: &str =
    "/cosmos.vesting.v1beta1.ContinuousVestingAccount";
const PERIODIC_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.PeriodicVestingAccount";
const DELAYED_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.DelayedVestingAccount";

pub const COMMUNITY_POOL_KEY: &str = "communitypool";
// Includes bonded and unbonded stake (rewards/commission)
pub const STAKING_BALANCE_KEY: &str = "staking";

/// Updates the cached total usomm balance of the foundation wallet
pub async fn update_foundation_balance(endpoint: &str) -> Result<()> {
    debug!("updating foundation wallet balance");
    match QueryClient::new(endpoint)?
        .balance(FOUNDATION_ADDRESS, USOMM)
        .await
    {
        Ok(b) => {
            let balance = b.balance.unwrap().amount;
            debug!("foundation wallet balance: {}usomm", balance);
            update_balance(FOUNDATION_ADDRESS, balance).await;
            return Ok(());
        }
        Err(e) => {
            bail!("error querying balance from endpoint {}: {:?}", endpoint, e);
        }
    }
}

/// Periodically updates the cached foundation balance
pub async fn poll_foundation_balance() -> Result<()> {
    let period = APP.config().cache.foundation_wallet_update_period;
    debug!(
        "updating foundation wallet balance every {} seconds",
        period
    );

    let config = APP.config();
    // jittered retry with exponential backoff
    let retry_strategy = ExponentialBackoff::from_millis(500)
        .map(jitter)
        .take(config.grpc.failed_query_retries as usize);
    loop {
        Retry::spawn(retry_strategy.clone(), || async {
            for endpoint in config.grpc.endpoints.iter() {
                match update_foundation_balance(&endpoint).await {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        warn!("{:?}", e);
                        continue;
                    }
                }
            }
            bail!("failed to query foundation wallet balance from all endpoints");
        })
        .await
        .unwrap_or_else(|e| error!("{:?}", e));
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Updates the cached total usomm balance in the community pool
pub async fn update_community_pool_balance(endpoint: &str) -> Result<()> {
    debug!("updating community pool balance");
    match QueryClient::new(endpoint)?.community_pool().await {
        Ok(r) => {
            let balance = get_dec_usomm_amount(r);
            debug!("community pool balance: {}usomm", balance);
            update_balance(COMMUNITY_POOL_KEY, balance).await;
            return Ok(());
        }
        Err(e) => {
            bail!(
                "error querying community pool from endpoint {}: {:?}",
                endpoint,
                e
            );
        }
    }
}

/// Periodically updates the cached community pool balance
pub async fn poll_community_pool_balance() -> Result<()> {
    let period = APP.config().cache.community_pool_update_period;
    debug!("updating community pool balance every {} seconds", period);

    let config = APP.config();
    // jittered retry with exponential backoff
    let retry_strategy = ExponentialBackoff::from_millis(500)
        .map(jitter)
        .take(config.grpc.failed_query_retries as usize);
    loop {
        Retry::spawn(retry_strategy.clone(), || async {
            for endpoint in config.grpc.endpoints.iter() {
                match update_community_pool_balance(&endpoint).await {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        warn!("{:?}", e);
                        continue;
                    }
                }
            }
            bail!("failed to query community pool balance from all endpoints");
        })
        .await
        .unwrap_or_else(|e| error!("{:?}", e));
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Updates the cached total usomm balance in the staking module. This includes both bonded
/// (staked/delegated) and unbonded (commission/rewards) funds.
pub async fn update_staking_balance(endpoint: &str) -> Result<()> {
    debug!("updating staking pool balance");
    match QueryClient::new(endpoint)?.pool().await {
        Ok(r) => {
            let pool = r.pool;
            let bonded: u128 = pool.clone().unwrap().bonded_tokens.parse()?;
            let unbonded: u128 = pool.unwrap().not_bonded_tokens.parse()?;
            let balance = bonded + unbonded;
            debug!("staking balance: {}usomm", balance);
            update_balance(STAKING_BALANCE_KEY, balance).await;
            return Ok(());
        }
        Err(e) => {
            bail!(
                "error querying staking pool from endpoint {}: {:?}",
                endpoint,
                e
            );
        }
    }
}

/// Periodically updates the cached staking pool balance
pub async fn poll_staking_balance() -> Result<()> {
    let period = APP.config().cache.staking_update_period;
    debug!("updating staking pool balance every {} seconds", period);

    let config = APP.config();
    // jittered retry with exponential backoff
    let retry_strategy = ExponentialBackoff::from_millis(500)
        .map(jitter)
        .take(config.grpc.failed_query_retries as usize);
    loop {
        Retry::spawn(retry_strategy.clone(), || async {
            for endpoint in config.grpc.endpoints.iter() {
                match update_staking_balance(&endpoint).await {
                    Ok(_) => return Ok(()),
                    Err(e) => {
                        warn!("{:?}", e);
                        continue;
                    }
                }
            }
            bail!("failed to query staking pool balance from all endpoints");
        })
        .await
        .unwrap_or_else(|e| error!("{:?}", e));
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Queries all accounts from the chain, filtering out somm1ymy6sx49d538gtdw2y6jnqwhcv3v9de8c92rql
/// which is the foundation address
/// Returns a vector of all accounts
pub async fn query_vesting_balance(endpoint: &str, address: &str) -> Result<u128> {
    let mut qclient = QueryClient::new(endpoint)?;
    let locked_balance: u128;
    let res = qclient.account_raw(address).await;
    if res.is_err() {
        bail!("error querying vesting account: {:?}", res);
    }

    let res = res.unwrap();
    let type_url = &res.type_url;
    let value: &[u8] = &res.value;
    if type_url == BASE_VESTING_ACCOUNT_TYPE_URL {
        let account = BaseVestingAccount::decode(value)?;
        locked_balance = get_usomm_amount(account.delegated_vesting);
    } else if type_url == CONTINUOUS_VESTING_ACCOUNT_TYPE_URL {
        let account = ContinuousVestingAccount::decode(value)?;
        locked_balance = get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
    } else if type_url == PERIODIC_VESTING_ACCOUNT_TYPE_URL {
        let account = PeriodicVestingAccount::decode(value)?;
        locked_balance = get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
    } else if type_url == DELAYED_VESTING_ACCOUNT_TYPE_URL {
        let account = DelayedVestingAccount::decode(value)?;
        locked_balance = get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
    } else {
        bail!(
            "the vesting account {} is of an unknown type: {}",
            address,
            type_url
        );
    }
    if locked_balance == 0 {
        warn!("{} has 0 locked", address);
    }
    Ok(locked_balance)
}

/// Periodically updates the cached total vesting balance
pub async fn poll_vesting_balance() -> Result<()> {
    let period = APP.config().cache.vesting_update_period;
    debug!("updating vesting balance every {} seconds", period);

    let config = APP.config();
    // jittered retry with exponential backoff
    let retry_strategy = ExponentialBackoff::from_millis(500)
        .map(jitter)
        .take(config.grpc.failed_query_retries as usize);
    loop {
        debug!("updating vesting balances");
        for address in VESTING_ACCOUNTS {
            Retry::spawn(retry_strategy.clone(), || async {
                for endpoint in config.grpc.endpoints.iter() {
                    match query_vesting_balance(&endpoint, address).await {
                        Ok(b) => {
                            update_balance(address, b).await;
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("{:?}", e);
                            continue;
                        }
                    }
                }

                bail!(
                    "failed to query vesting balance of {} from all endpoints",
                    address
                );
            })
            .await
            .unwrap_or_else(|e| error!("{:?}", e));
        }
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Converts [`Vec<Coin>`] to the sum of the contained usomm amounts
pub fn get_usomm_amount(coins: Vec<Coin>) -> u128 {
    coins
        .iter()
        .filter_map(|c| {
            if c.denom == USOMM {
                Some(c.amount.parse::<u128>().unwrap())
            } else {
                None
            }
        })
        .sum()
}

pub fn get_dec_usomm_amount(coins: Vec<DecCoin>) -> u128 {
    coins
        .iter()
        .filter_map(|c| {
            if c.denom == USOMM {
                let truncated = &c.amount[0..c.amount.len() - 18];
                Some(truncated.parse::<u128>().unwrap())
            } else {
                None
            }
        })
        .sum()
}

pub async fn update_balance(key: &str, value: u128) {
    BALANCES.lock().await.insert(key.to_string(), value);
}
//
