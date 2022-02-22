.PHONY: test build deploy install

test:
	cargo build
	cargo test -- --nocapture
	python3 ./cli/test/main.py

build:
	cargo build --release --manifest-path ./cli/Cargo.toml

deploy:
	cargo build-bpf --manifest-path ./programs/nft/Cargo.toml
	solana program deploy ./target/deploy/chill_nft.so --url devnet

deploy-mainnet:
	cargo build-bpf --manifest-path ./programs/nft/Cargo.toml
	solana program deploy ./target/deploy/chill_nft.so --url mainnet

install:
	cargo install --path ./cli
