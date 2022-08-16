FROM rust:1.62-slim-bullseye as build

ARG DEBIAN_FRONTEND=noninteractive
RUN apt update && apt -qq upgrade -y && apt install -qq pkg-config libssl-dev -y && apt clean && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash rust
USER rust

WORKDIR /home/rust/
RUN cargo new finnhub_ws
WORKDIR /home/rust/finnhub_ws

COPY ./Cargo.lock ./Cargo.toml ./

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm -rf target/
RUN cargo build --release

RUN cargo test


FROM debian:bullseye as final
ARG DEBIAN_FRONTEND=noninteractive
RUN apt update && apt -qq upgrade -y && apt -qq install libssl-dev ca-certificates -y && update-ca-certificates && apt clean && rm -rf /var/lib/apt/lists/*
# copy the build artifact from the build stage
COPY --from=build /home/rust/finnhub_ws/target/release/finnhub_ws .

# set the startup command to run your binary
ENTRYPOINT ["./finnhub_ws"]

CMD ["--help"]
