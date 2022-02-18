import json
from pathlib import Path
from solana.keypair import Keypair
from solana.publickey import PublicKey

PROJECT_DIR = Path(__file__).parent.parent.parent
EXECUTABLE_NAME = 'chill-cli'
EXECUTABLE_PATH = PROJECT_DIR / 'target' / 'debug' / EXECUTABLE_NAME
DEFAULT_KEY_PATH = Path.home() / '.config' / 'solana' / 'id.json'
DEFAULT_MINT_PATH = Path.cwd() / 'mint.devnet.pubkey'

KEYPAIRS = PROJECT_DIR / 'localnet'
AUTHORITY_PATH = KEYPAIRS / 'authority.json'
RECIPIENT_PATH = KEYPAIRS / 'recipient.json'
TESTMINT_PATH = KEYPAIRS / 'mint.pubkey.localnet'


def get_mint_pubkey(path):
    with open(path, 'r', encoding='UTF-8') as file:
        pubkey = file.read()
        return PublicKey(pubkey)


def get_keypair(path):
    with open(path, 'r', encoding='UTF-8') as file:
        keypair = json.load(file)
        keypair = bytes([int(i) for i in keypair])
        return Keypair.from_secret_key(keypair)


def default_authority():
    path = Path.home() / '.config' / 'solana' / 'id.json'
    return get_keypair(path).public_key


def default_mintfile():
    path = Path.cwd() / 'mint.devnet.pubkey'
    return get_mint_pubkey(path)


def authority():
    return get_keypair(AUTHORITY_PATH).public_key


def recipient_pubkey():
    return get_keypair(RECIPIENT_PATH).public_key


def testmint_pubkey():
    return get_mint_pubkey(TESTMINT_PATH)
