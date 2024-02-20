# sommstats

A convenience API for Sommelier state.

## Getting Started

The minimal configuration needed to run is a list of grpc endpoints:

```toml
[grpc]
endpoints = [
    "https://sommelier-grpc.polkachu.com:14190",
    "https://sommelier.archive.strange.love:9090"
]
```

To run with cargo:

```bash
cargo run -- -c <config toml path> start
```


## API

Right now there is only one functioning endpoint `/v1/circulating-supply`. A request to `/` will return an empty response with a 200 status code. If any balances have not been loaded into the cache (i.e. the service is starting up), a 503 will be returned. Otherwise, a simple response with a body of the circulating supply in SOMM will be returned:

```
1234567890
```

Units are in `SOMM`, no conversion is needed.

## Config

Default config values are equivalent to the following config file:

```toml
[grpc]
endpoints = []
# number of times a failed query should be retried each period
failed_query_retries = 3

[server]
address = "0.0.0.0"
port = 8080

[cache]
# how frequently the cache should refresh the respective balance(s)
community_pool_update_period = 3600
vesting_update_period = 3600
foundation_wallet_update_period = 3600
```


[Documentation]

[Abscissa]: https://github.com/iqlusioninc/abscissa
[Documentation]: https://docs.rs/abscissa_core/
