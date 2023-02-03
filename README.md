# sommstats

An API for Sommelier statistics


## Getting Started

The minimal configuration needed to run is a list of grpc endpoints:

```toml
[grpc]
endpoints = [
    "https://sommelier-grpc.polkachu.com:14190",
    "https://sommelier-grpc.lavenderfive.com:443",
    "https://grpc.somm.bh.rocks:443/",
    "https://sommelier.archive.strange.love:9090"
]
```

To run with cargo:

```bash
cargo run -- -c <config toml path> start
```

To build and run container locally:

```bash
# cwd is root of this directory
make
docker run -p 3000:443 -it sommstats:prebuilt
```


## API

Right now there is only one endpoint `/api/v1/circulating-supply`. If any balances have not been loaded into the cache, a 503 will be returned. Otherwise a response of the following form will be returned:

```json
{"circulating_supply": 1234567890}
```

Units are `usomm`

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
staking_update_period = 3600
vesting_update_period = 3600
foundation_wallet_update_period = 3600
```


[Documentation]

[Abscissa]: https://github.com/iqlusioninc/abscissa
[Documentation]: https://docs.rs/abscissa_core/
