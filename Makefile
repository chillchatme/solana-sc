.PHONY: test build deploy deploy-mainnet install

test:
	cargo test
	yarn
	yarn run anchor build -p chill_wallet
	yarn run anchor build -p chill_nft
	yarn run -- anchor build -p chill_staking -- --features short-day
	yarn run anchor test --skip-build
	yarn run anchor build -p chill_staking
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
	yarn run anchor deploy --provider.cluster devnet

deploy-mainnet:
	yarn
	yarn run anchor build
	yarn run anchor deploy --provider.cluster mainnet

install:
	cargo install --path ./cli
