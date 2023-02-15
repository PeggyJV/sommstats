# Reference: https://www.lpalmieri.com/posts/fast-rust-docker-builds/

FROM rust:1.63 as cargo-chef-rust
RUN cargo install cargo-chef --version 0.1.51

FROM cargo-chef-rust as planner
WORKDIR /app
# We only pay the installation cost once,
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning
# the cargo-chef version with `--version X.X.X`
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM cargo-chef-rust as cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN rustup component add rustfmt
RUN cargo chef cook --release --recipe-path recipe.json

FROM cargo-chef-rust as builder
WORKDIR /app
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:3.17.2 as runtime
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/sommstats /usr/local/bin
COPY ./configs/prod_config.toml ./config.toml
CMD sommstats -c config.toml start
