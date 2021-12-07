# Bein Chain

[![Substrate version](https://img.shields.io/badge/Substrate-4.0.0--dev-blue?logo=Parity%20Substrate)](https://substrate.dev/)
[![No License](https://img.shields.io/apm/l/License?label=License)](https://github.com/Ylgr/bein/blob/master/LICENSE)

Bein Chain is a Substrate-based Blockchain platform with a mission together with the social network Bein Group (expected to launch in the third quarter of 2022) to bring Blockchain to everyone to gradually educate users about Blockchain.

## Introducing Reef chain

Bein chain is an EVM compatible blockchain for DeFi. In Bein Chain you can have:

- <strong>Solidity</strong>: Port your existing applications from Ethereum (and other EVM-compatible platforms) to Bein Chain without modifying your Solidity code. You can also use Ethereum-based development tools (like Remix, Metamask, Hardhat, etc.) to build products on the Bein Chain platform.

- <strong>Social token</strong>: You can fully deploy your social tokens and NFTs on Bein Chain and easily integrate it into the Bein social networking platform from which to build your personal brand.

- <strong>Dapps as extension</strong>: Extensions on the Bein Group social network will be built as dapps on Bein Chain. You can imagine you are an artist who just released an exclusive NFT, after creating NFT you drag an NFT auction house into your own Group. Thanks to that, your exclusive products easily reach the people who trust you most.

- <strong>Feeless tips</strong>: Overcome the fear of transaction fees, your community will now give you all the love thanks to a staking system that receives a bandwidth for making transactions with no fees.

## Getting Started

### Rust Setup

First, complete the [basic Rust setup instructions](./doc/rust-setup.md).

### Run

Use Rust's native `cargo` command to build and launch the template node:

```sh
cargo run --release -- --dev --tmp
```

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/bein -h
```

## Run

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/bein --dev
```

Purge the development chain's state:

```bash
./target/release/bein purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_BACKTRACE=1 ./target/release/bein -ldebug --dev
```

### Connect with Polkadot-JS Apps Front-end

Once the node template is running locally, you can connect it with **Polkadot-JS Apps** front-end
to interact with your chain. [Click
here](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) connecting the Apps to your
local Bein Chain.

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, refer to our
[Start a Private Network tutorial](https://substrate.dev/docs/en/tutorials/start-a-private-network/).
