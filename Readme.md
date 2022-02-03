# Chill CLI

## Installation

Make sure you have installed [Rust
toolchain](https://www.rust-lang.org/tools/install).

Run the command:

``` bash
cargo build --release
```

The executable will be located in `./target/release/chill-cli`.

## Usage

By default all commands run in
[Devnet](https://docs.solana.com/ru/clusters#devnet) where you might
test your application without paying any money. To mint 1000 tokens, you
can type:

``` bash
./chill-cli mint 1000
```

You can mint tokens with this command as many times as you want.

To check your balance, type:

``` bash
./chill-cli balance
```

If you want to mint tokens to the
[Mainnet](https://docs.solana.com/ru/clusters#mainnet-beta), you should
first top up your Solana wallet balance.

``` bash
# If your wallet is placed in the default location ~/.config/solana/id.json
./chill-cli mint 123.456 --mainnet

# If the wallet is placed somewhere else, you should specify the correct path
./chill-cli mint 123.456 --mainnet --owner <PATH>
```

For more info, run:

``` bash
./chill-cli --help
```
