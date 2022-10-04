# FinnHub stocks analysis  
This project is implemented as a part of a homework exercise for [077] - Real Time Embedded Systems
course of ECE Department, AUTh.

This fetches stock information changes as they happen in real time, using websockets, runs some analysis
and persists the information as it becomes available. It should consume the least cpu it can as it should
be able to be deployed in a microcontroller. 


## Getting Started

### Prerequisites
1. Rust

To install them on variant Linux distributions follow the instructions below

#### Fedora
```shell
$ sudo dnf upgrade --refresh # updates installed packages and repositories metadata
$ sudo dnf install rust openssl-devel # or use rustup https://www.rust-lang.org/tools/install
```

#### Ubuntu
```shell
$ sudo apt-get update && sudo apt-get upgrade # updates installed packages and repositories metadata
$ sudo apt-get install rust libssl-dev # or use rustup https://www.rust-lang.org/tools/install 
```


### Build & Run
First, go to [finnhub.io](https://finnhub.io/) and get a free API Key. Save it for later use.

Then to build the application:
1. Compile the project
    ```shell
    $ cargo build --release # if you want a development build just drop the release flag.  
    ```
   To cross-compile for Raspberry Pi, you gotta install the toolchain via rustup as most package managers will 
   only ship you the toolchain for your machine's architecture.
   ```shell
   $ cargo build --release --target=aarch64-unknown-linux-gnu 
   ```
   If you are compiling on the RP, just run the first command. Rust compiler should pick your architecture automatically.
2. Run the binary
    ```shell
    $ ./target/release/finnhub_ws  --token <your-finnhub-token> --stocks <stockSymbol> --stocks <stockSymbol>
    ```

