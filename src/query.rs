use eyre::{bail, Result};
use ocular::{
    cosmrs::proto::{
        cosmos::{
            base::v1beta1::Coin,
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
    accounts::VESTING_ACCOUNTS,
    application::{BALANCES, USOMM},
};

const BASE_ACCOUNT_TYPE_URL: &str = "/cosmos.auth.v1beta1.BaseAccount";
const MODULE_ACCOUNT_TYPE_URL: &str = "/cosmos.auth.v1beta1.ModuleAccount";
const BASE_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.BaseVestingAccount";
const CONTINUOUS_VESTING_ACCOUNT_TYPE_URL: &str =
    "/cosmos.vesting.v1beta1.ContinuousVestingAccount";
const PERIODIC_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.PeriodicVestingAccount";
const DELAYED_VESTING_ACCOUNT_TYPE_URL: &str = "/cosmos.vesting.v1beta1.DelayedVestingAccount";

/// Queries all accounts from the chain, filtering out somm1ymy6sx49d538gtdw2y6jnqwhcv3v9de8c92rql
/// which is the foundation address
/// Returns a vector of all accounts
pub async fn track_vesting_balances(qclient: &mut QueryClient) -> Result<()> {
    for address in VESTING_ACCOUNTS {
        let res = qclient.account_raw(address).await;
        if res.is_err() {
            bail!("error querying all accounts: {:?}", res);
        }

        let res = res.unwrap();
        let locked_balance: u128;
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

        update_balance(address, locked_balance).await;
    }

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

pub async fn update_balance(key: &str, value: u128) {
    BALANCES.lock().await.insert(key.to_string(), value);
}
