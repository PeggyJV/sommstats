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
    accounting::{FOUNDATION_ADDRESS, TOTAL_USOMM_SUPPLY, VESTING_ACCOUNTS},
    application::{BALANCES, USOMM},
};

const BASE_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.BaseVestingAccount";
const CONTINUOUS_VESTING_ACCOUNT_TYPE_URL: &str =
    "/cosmos.vesting.v1beta1.ContinuousVestingAccount";
const PERIODIC_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.PeriodicVestingAccount";
const DELAYED_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.DelayedVestingAccount";

const COMMUNITY_POOL_KEY: &str = "communitypool";
// Includes bonded and unbonded stake (rewards/commission)
const STAKING_BALANCE_KEY: &str = "staking";
const VESTING_BALANCE_KEY: &str = "vesting";

/// Circulating supply == Total supply - Foundation wallet - Staking - Community Pool - Vesting balances
pub async fn get_circulating_supply() -> String {
    let balances = BALANCES.lock().await;
    (TOTAL_USOMM_SUPPLY
        - balances.get(FOUNDATION_ADDRESS).unwrap()
        - balances.get(STAKING_BALANCE_KEY).unwrap()
        - balances.get(COMMUNITY_POOL_KEY).unwrap()
        - balances.get(VESTING_BALANCE_KEY).unwrap())
    .to_string()
}

/// Updates the cached total usomm balance in the community pool
pub async fn update_foundation_balance(qclient: &mut QueryClient) -> Result<()> {
    let res = qclient.balance(FOUNDATION_ADDRESS, USOMM).await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let res = res.unwrap();

    update_balance(FOUNDATION_ADDRESS, res.balance.unwrap().amount).await;

    Ok(())
}

/// Updates the cached total usomm balance in the community pool
pub async fn update_community_pool_balance(qclient: &mut QueryClient) -> Result<()> {
    let res = qclient.community_pool().await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let res = res.unwrap();
    update_balance(COMMUNITY_POOL_KEY, get_dec_usomm_amount(res)).await;

    Ok(())
}

/// Updates the cached total usomm balance in the staking module. This includes both bonded
/// (staked/delegated) and unbonded (commission/rewards) funds.
pub async fn update_staking_balance(qclient: &mut QueryClient) -> Result<()> {
    let res = qclient.pool().await;
    if res.is_err() {
        bail!("error querying all accounts: {:?}", res);
    }

    let res = res.unwrap();
    let bonded: u128 = res.pool.clone().unwrap().bonded_tokens.parse()?;
    let unbonded: u128 = res.pool.unwrap().not_bonded_tokens.parse()?;

    update_balance(STAKING_BALANCE_KEY, bonded + unbonded).await;

    Ok(())
}

/// Queries all accounts from the chain, filtering out somm1ymy6sx49d538gtdw2y6jnqwhcv3v9de8c92rql
/// which is the foundation address
/// Returns a vector of all accounts
pub async fn update_vesting_balance(qclient: &mut QueryClient) -> Result<()> {
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

    update_balance(VESTING_BALANCE_KEY, locked_balance).await;

    Ok(())
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
