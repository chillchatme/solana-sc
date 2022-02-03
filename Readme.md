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

The `mint` command creates a file named `mint.pubkey` in the same
directory from which the command was run. This file will contain the
public key of your mint account. If you delete the `mint.pubkey` file,
rename it, move it to another directory, the next time you run the
`./chill-cli mint` command, it will generate a new mint and save it
again. To prevent this behavior, you can explicitly specify the path to
the file or the public key in the base58 encoding with the argument
`--mint-address`.

For example:

``` bash
./chill-cli mint 1000 --mint-address ./mint.pubkey
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
```

For more information, run:

``` bash
./chill-cli --help
```
