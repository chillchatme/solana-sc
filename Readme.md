# Chill

## Installation

Make sure you have installed [Rust
toolchain](https://www.rust-lang.org/tools/install), [Solana Tool
Suite](https://docs.solana.com/cli/install-solana-cli-tools), and GNU Make
utility.

Run the command to build CLI and smart constracts:

```bash
make build
```

The CLI executable will be located in `./target/release/chill-cli`.

## Deploying

Run `make build` command to build smart contracts and generate keypairs in the
`./target/deploy` directory. Replace pubkeys in the files `lib.rs` of the
corresponding smart constracts and in the file `Anchor.toml`. You might to get
a public key from a keypair using the `solana-keygen pubkey` command.

After updating pubkeys, run `make deploy` command to deploy the smart contracts
in the Devnet cluster of the blockchain or `make deploy-mainnet` to deploy it
in the Mainnet cluster for SOL.

## Testing

Run this commands to test:

```bash
make test
```

## Usage

By default, all commands run in
[Devnet](https://docs.solana.com/ru/clusters#devnet) where you might test your
application without paying any money. To mint 1000 tokens, you can type:

```bash
./chill-cli mint 1000
```

You can mint tokens with this command as many times as you want.

The `mint` command creates a file named `mint.<cluster>.pubkey` in the same
directory from which the command was run. This file will contain the public key
of your mint account. If you delete the `mint.<cluster>.pubkey` file, rename
it, move it to another directory, the next time you run the `./chill-cli mint`
command, it will generate a new mint and save it again. To prevent this
behavior, you can explicitly specify the path to the file or the public key in
the base58 encoding with the argument `--mint-address`.

For example:

```bash
./chill-cli mint 1000 --mint-address ./mint.devnet.pubkey
./chill-cli mint 1000 --mint-address CSqhdWtH9Zk5GEEdHFHQHFic8RdTxfMkEoCPevBK1PTL
```

To check your balance, type:

```bash
./chill-cli balance
```

If you want to mint tokens to the
[Mainnet](https://docs.solana.com/ru/clusters#mainnet-beta), you should first
top up your Solana wallet balance.

```bash
# If your wallet is placed in the default location ~/.config/solana/id.json
./chill-cli mint 123.456 --url mainnet

# If the wallet is placed somewhere else, you should specify the correct path
./chill-cli mint 123.456 --url mainnet --owner <PATH>

# Specify mint file explicitly
./chill-cli mint 123.456 --url mainnet --mint-address ./mint.mainnet.pubkey
```

To transfer tokens, type:

```bash
./chill-cli transfer <ACCOUNT_ADDRESS> <AMOUNT>

# For example
./chill-cli transfer CbPL8HynuwoheoxpztiUJZpVVuHnt9SvFNnBt2UBwxW2 100

# Transfer in Mainnet
./chill-cli transfer "~/.config/solana/recipient.json" 100 --url mainnet
```

Keep in mind, if the recipient does not have any token account for your mint,
the transfer command will create one at your expense. It will cost about
`0.002` SOL.

If you have minted a NFT to yourself, and then transfer it to someone, the
recipient will be added to a creators list.

To initialize the Chill smart contract, you should run:

```bash
./chill-cli initialize        \
    --character <FEES>        \
    --emote <FEES>            \
    --item <FEES>             \
    --pet <FEES>              \
    --tileset <FEES>          \
    --world <FEES>            \
    --recipient <ADDRESS>
```

Use the program ID that was printed during deployment.

This command initializes the smart contract. Each argument with \<FEES\> means
the price in Chill tokens to mint a NFT of this type. A recipient is an address
who receives fees.

If you want to add more recipients, you might specify them (up to 3 recipients)
with the corresponded share. The share is percentage number (all mint and
transaction shares must sum up to 100).

```bash
./chill-cli initialize          \
    --character <FEES>          \
    ...                         \
    --recipient <ADDRESS_1>     \
    --mint-share <SHARE>        \
    --transaction-share <SHARE> \
    --recipient <ADDRESS_2>     \
    --mint-share <SHARE>        \
    --transaction-share <SHARE>
```

You can mint NFT tokens with this command:

```bash
./chill-cli mint-nft <TYPE> <NAME> <URI>

# Example
./chill-cli mint-nft pet "Bob the cat" https://arweave.org/hkjc8h3jk2938hk32
```

You can initialize staking account with this commands:

```bash
./chill-cli staking initialize     \
    --start "2023-01-01T00:00:00Z" \
    --end "2027-12-31T00:00:00Z"   \
    --min-stake-size 1.500

./chill-cli staking add-reward-tokens 123.456
```

For more information, run:

```bash
./chill-cli --help
```
