.PHONY: test build deploy install

test:
	cargo test
	python ./cli/test/main.py

build:
	cargo build --release --manifest-path ./cli/Cargo.toml

deploy:
	cargo build-bpf --manifest-path ./programs/nft/Cargo.toml
	solana program deploy ./target/deploy/chill_nft.so

install:
	cargo install --path ./cli