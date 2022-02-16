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

The `mint` command creates a file named `mint.\<cluster\>.pubkey` in the
same directory from which the command was run. This file will contain
the public key of your mint account. If you delete the
`mint.\<cluster\>.pubkey` file, rename it, move it to another directory,
the next time you run the `./chill-cli mint` command, it will generate a
new mint and save it again. To prevent this behavior, you can explicitly
specify the path to the file or the public key in the base58 encoding
with the argument `--mint-address`.

For example:

``` bash
./chill-cli mint 1000 --mint-address ./mint.devnet.pubkey
./chill-cli mint 1000 --mint-address CSqhdWtH9Zk5GEEdHFHQHFic8RdTxfMkEoCPevBK1PTL
```

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

# Specify mint file explicitly
./chill-cli mint 123.456 --mainnet --mint-address ./mint.mainnet.pubkey
```

To transfer tokens, type:

``` bash
./chill-cli transfer <ACCOUNT_ADDRESS> <AMOUNT>

# For example
./chill-cli transfer CbPL8HynuwoheoxpztiUJZpVVuHnt9SvFNnBt2UBwxW2 100

# Transfer in Mainnet
./chill-cli transfer "~/.config/solana/receiver.json" 100 --mainnet
```

Keep in mind, if the recipient does not have any token account for your
mint, the transfer command will create one at your expense. It will cost
about `0.002` SOL.

For more information, run:

``` bash
./chill-cli --help
```
