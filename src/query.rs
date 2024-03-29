use abscissa_core::{
    tracing::{
        debug,
        log::{error, info, warn},
    },
    Application,
};
use abscissa_tokio::tokio;
use chrono::Utc;
use eyre::{bail, Result};
use ocular::{
    cosmrs::proto::{
        cosmos::{
            base::v1beta1::{Coin, DecCoin},
            vesting::v1beta1::{
                ContinuousVestingAccount, DelayedVestingAccount, PeriodicVestingAccount,
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
    accounting::{FOUNDATION_ADDRESS, FOUNDATION_ADDRESS_2, VESTING_ACCOUNTS},
    application::{BALANCES, USOMM},
    prelude::APP,
};

const _BASE_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.BaseVestingAccount";
const CONTINUOUS_VESTING_ACCOUNT_TYPE_URL: &str =
    "/cosmos.vesting.v1beta1.ContinuousVestingAccount";
const PERIODIC_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.PeriodicVestingAccount";
const DELAYED_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.DelayedVestingAccount";

pub const COMMUNITY_POOL_KEY: &str = "communitypool";

/// Updates the cached total usomm balance of the foundation wallet
pub async fn update_foundation_balance(endpoint: &str) -> Result<()> {
    match QueryClient::new(endpoint)?
        .balance(FOUNDATION_ADDRESS, USOMM)
        .await
    {
        Ok(b) => {
            let balance = b.balance.unwrap().amount as u64;
            update_balance(FOUNDATION_ADDRESS, balance).await;
            info!("foundation wallet balance updated: {}usomm", balance);

            Ok(())
        }
        Err(e) => {
            bail!(
                "error querying foundation wallet balance from endpoint {}: {:?}",
                endpoint,
                e
            );
        }
    }
}

/// Updates the cached total usomm balance of the foundation wallet
pub async fn update_foundation_balance_2(endpoint: &str) -> Result<()> {
    match QueryClient::new(endpoint)?
        .balance(FOUNDATION_ADDRESS_2, USOMM)
        .await
    {
        Ok(b) => {
            let balance = b.balance.unwrap().amount as u64;
            update_balance(FOUNDATION_ADDRESS_2, balance).await;
            info!("foundation wallet 2 balance updated: {}usomm", balance);

            Ok(())
        }
        Err(e) => {
            bail!(
                "error querying foundation wallet 2 balance from endpoint {}: {:?}",
                endpoint,
                e
            );
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
        debug!("updating foundation wallet balance");
        Retry::spawn(retry_strategy.clone(), || async {
            for endpoint in config.grpc.endpoints.iter() {
                if let Err(e) = update_foundation_balance(endpoint).await {
                    warn!("{e:?}");
                    continue;
                }

                if let Err(e) = update_foundation_balance_2(endpoint).await {
                    warn!("{e:?}");
                    continue;
                }

                return Ok(());
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
    match QueryClient::new(endpoint)?.community_pool().await {
        Ok(r) => {
            let balance = get_dec_usomm_amount(r);
            update_balance(COMMUNITY_POOL_KEY, balance).await;
            info!("community pool balance updated: {}usomm", balance);

            Ok(())
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
        debug!("updating community pool balance");
        Retry::spawn(retry_strategy.clone(), || async {
            for endpoint in config.grpc.endpoints.iter() {
                if let Err(e) = update_community_pool_balance(endpoint).await {
                    warn!("{e:?}");
                    continue;
                }

                return Ok(());
            }

            bail!("failed to query community pool balance from all endpoints");
        })
        .await
        .unwrap_or_else(|e| error!("{:?}", e));
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Queries the balance of the account, which is assumed to be a vesting account, and returns
/// the portion of the balance that is still vesting (locked)
pub async fn query_vesting_balance(endpoint: &str, address: &str) -> Result<u64> {
    let mut qclient = QueryClient::new(endpoint)?;
    let res = qclient.account_raw(address).await?;
    let current_time = Utc::now().timestamp();
    let type_url = &res.type_url;
    let value: &[u8] = &res.value;

    debug!("current time: {current_time}");

    // get the still-vesting (locked) balance of the account
    let locked_balance = match type_url.as_str() {
        CONTINUOUS_VESTING_ACCOUNT_TYPE_URL => {
            let account = ContinuousVestingAccount::decode(value)?;

            debug!(
                "continuous account start time: {} end time: {}",
                account.start_time,
                account.base_vesting_account.clone().unwrap().end_time
            );
            if account.start_time > current_time {
                0_u64
            } else {
                let base = account.base_vesting_account.clone().unwrap();
                let original_vesting = get_usomm_amount(base.original_vesting);
                let unlocked_proportion = (current_time - account.start_time) as f64
                    / (base.end_time - account.start_time) as f64;

                (original_vesting as f64 * (1.0 - unlocked_proportion)) as u64
            }
        }
        PERIODIC_VESTING_ACCOUNT_TYPE_URL => {
            let account = PeriodicVestingAccount::decode(value)?;
            let periods = account.vesting_periods;
            let mut locked_balance: u64 = 0;

            debug!("periodic account start time: {}", account.start_time);
            let mut start_time = account.start_time;
            for period in periods {
                debug!(
                    "period end time: {}, period length: {}",
                    start_time + period.length,
                    period.length
                );
                locked_balance += if current_time > start_time + period.length {
                    0
                } else {
                    get_usomm_amount(period.amount)
                };

                start_time += period.length;
            }

            locked_balance
        }
        DELAYED_VESTING_ACCOUNT_TYPE_URL => {
            let account = DelayedVestingAccount::decode(value)?;
            let base = account.base_vesting_account.unwrap();

            debug!("delayed vesting account end time: {}", base.end_time);
            let locked_balance = if current_time > base.end_time {
                0
            } else {
                get_usomm_amount(base.original_vesting)
            };

            debug!("delayed vesting account locked balance {locked_balance}");
            locked_balance
        }
        _ => {
            bail!(
                "vesting account {} is of an unhandled type: {}",
                address,
                type_url
            );
        }
    };

    info!("locked balance for {address} is {locked_balance}");

    // so we can remove the address from the query list when it's done vesting
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
                    match query_vesting_balance(endpoint, address).await {
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
pub fn get_usomm_amount(coins: Vec<Coin>) -> u64 {
    coins
        .iter()
        .filter_map(|c| {
            if c.denom == USOMM {
                Some(c.amount.parse::<u64>().unwrap())
            } else {
                None
            }
        })
        .sum()
}

pub fn get_dec_usomm_amount(coins: Vec<DecCoin>) -> u64 {
    coins
        .iter()
        .filter_map(|c| {
            if c.denom == USOMM {
                let truncated = &c.amount[0..c.amount.len() - 18];
                Some(truncated.parse::<u64>().unwrap())
            } else {
                None
            }
        })
        .sum()
}

pub async fn update_balance(key: &str, value: u64) {
    BALANCES.lock().await.insert(key.to_string(), value);
}
//
