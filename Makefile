.PHONY: test build deploy install

test:
	cargo test
	python ./chill-cli/test/main.py

build:
	cargo build --release --manifest-path ./chill-cli/Cargo.toml

deploy:
	cargo build-bpf --manifest-path ./chill-program/Cargo.toml
	solana program deploy ./target/deploy/chill_program.so

install:
	cargo install --path ./chill-cli
