use std::{collections::HashMap, fs, path::Path};

use eyre::Result;
use serde::{Deserialize, Serialize};
use time;

use crate::application::BALANCES;

const SNAPSHOT_FILE: &str = "snapshot.json";

#[derive(Debug, Deserialize, Serialize)]
struct Snapshot {
    timestamp: time::OffsetDateTime,
    cache: HashMap<String, u128>,
}

pub(crate) async fn take_cache_snapshot() -> Result<()> {
    let cache: HashMap<String, u128> = BALANCES.lock().await.clone();
    let snapshot_json = serde_json::to_string::<Snapshot>(&Snapshot {
        timestamp: time::OffsetDateTime::now_utc(),
        cache,
    })?;

    fs::write(SNAPSHOT_FILE, &snapshot_json)?;

    Ok(())
}

pub(crate) async fn try_load_snapshot() -> Result<bool> {
    if !Path::new(SNAPSHOT_FILE).exists() {
        return Ok(false)
    }

    let snapshot = fs::read(SNAPSHOT_FILE)?;
    let mut cache = BALANCES.lock().await;
    cache.extend(serde_json::from_slice::<Snapshot>(&snapshot)?.cache);

    Ok(true)
}
