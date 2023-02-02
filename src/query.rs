use abscissa_core::{tracing::debug, Application};
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
pub const VESTING_BALANCE_KEY: &str = "vesting";

/// Updates the cached total usomm balance in the community pool
pub async fn update_foundation_balance() -> Result<()> {
    debug!("updating foundation wallet balance");
    let mut qclient = QueryClient::new(&APP.config().grpc)?;
    let res = qclient.balance(FOUNDATION_ADDRESS, USOMM).await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let balance = res.unwrap().balance.unwrap().amount;
    debug!("foundation wallet balance: {}usomm", balance);
    update_balance(FOUNDATION_ADDRESS, balance).await;

    Ok(())
}

/// Periodically updates the cached foundation balance
pub async fn poll_foundation_balance() -> Result<()> {
    let period = APP.config().cache.foundation_wallet_update_period;
    debug!(
        "updating foundation wallet balance every {} seconds",
        period
    );
    loop {
        update_foundation_balance().await?;
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Updates the cached total usomm balance in the community pool
pub async fn update_community_pool_balance() -> Result<()> {
    debug!("updating community pool balance");
    let mut qclient = QueryClient::new(&APP.config().grpc)?;
    let res = qclient.community_pool().await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let balance = get_dec_usomm_amount(res.unwrap());
    debug!("community pool balance: {}usomm", balance);
    update_balance(COMMUNITY_POOL_KEY, balance).await;

    Ok(())
}

/// Periodically updates the cached community pool balance
pub async fn poll_community_pool_balance() -> Result<()> {
    let period = APP.config().cache.community_pool_update_period;
    debug!("updating community pool balance every {} seconds", period);
    loop {
        update_community_pool_balance().await?;
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Updates the cached total usomm balance in the staking module. This includes both bonded
/// (staked/delegated) and unbonded (commission/rewards) funds.
pub async fn update_staking_balance() -> Result<()> {
    debug!("updating staking pool balance");
    let mut qclient = QueryClient::new(&APP.config().grpc)?;
    let res = qclient.pool().await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let res = res.unwrap();
    let bonded: u128 = res.pool.clone().unwrap().bonded_tokens.parse()?;
    let unbonded: u128 = res.pool.unwrap().not_bonded_tokens.parse()?;
    let balance = bonded + unbonded;
    debug!("staking balance: {}usomm", balance);
    update_balance(STAKING_BALANCE_KEY, balance).await;

    Ok(())
}

/// Periodically updates the cached staking pool balance
pub async fn poll_staking_balance() -> Result<()> {
    let period = APP.config().cache.staking_update_period;
    debug!("updating staking pool balance every {} seconds", period);
    loop {
        update_staking_balance().await?;
        tokio::time::sleep(std::time::Duration::from_secs(period)).await;
    }
}

/// Queries all accounts from the chain, filtering out somm1ymy6sx49d538gtdw2y6jnqwhcv3v9de8c92rql
/// which is the foundation address
/// Returns a vector of all accounts
pub async fn update_vesting_balance() -> Result<()> {
    debug!("updating total vesting balance");
    let mut qclient = QueryClient::new(&APP.config().grpc)?;
    let mut locked_balance: u128 = 0;
    for address in VESTING_ACCOUNTS {
        let res = qclient.account_raw(address).await;
        if res.is_err() {
            bail!("error querying all accounts: {:?}", res);
        }

        let res = res.unwrap();
        let type_url = &res.type_url;
        let value: &[u8] = &res.value;
        if type_url == BASE_VESTING_ACCOUNT_TYPE_URL {
            let account = BaseVestingAccount::decode(value)?;
            locked_balance = locked_balance + get_usomm_amount(account.delegated_vesting);
        } else if type_url == CONTINUOUS_VESTING_ACCOUNT_TYPE_URL {
            let account = ContinuousVestingAccount::decode(value)?;
            locked_balance = locked_balance
                + get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
        } else if type_url == PERIODIC_VESTING_ACCOUNT_TYPE_URL {
            let account = PeriodicVestingAccount::decode(value)?;
            locked_balance = locked_balance
                + get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
        } else if type_url == DELAYED_VESTING_ACCOUNT_TYPE_URL {
            let account = DelayedVestingAccount::decode(value)?;
            locked_balance = locked_balance
                + get_usomm_amount(account.base_vesting_account.unwrap().delegated_vesting);
        } else {
            bail!(
                "the vesting account {} is of an unknown type: {}",
                address,
                type_url
            );
        }
    }

    debug!("vesting (locked) balance: {}usomm", locked_balance);
    update_balance(VESTING_BALANCE_KEY, locked_balance).await;

    Ok(())
}

/// Periodically updates the cached total vesting balance
pub async fn poll_vesting_balance() -> Result<()> {
    let period = APP.config().cache.vesting_update_period;
    debug!("updating vesting balance every {} seconds", period);
    loop {
        update_vesting_balance().await?;
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
