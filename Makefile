.PHONY: test build deploy deploy-mainnet install

test:
	yarn
	yarn run anchor test
	# cargo build --release --manifest-path ./cli/Cargo.toml
	# python3 -m pip install -r ./requirements.txt
	# python3 ./cli/test/main.py

build:
	yarn
	yarn run anchor build
	cargo build --release --manifest-path ./cli/Cargo.toml

deploy:
	yarn
	yarn run anchor build
	yarn run anchor deploy --provider.cluster testnet

deploy-mainnet:
	yarn
	yarn run anchor build
	yarn run anchor deploy --provider.cluster mainnet

install:
	cargo install --path ./cli
