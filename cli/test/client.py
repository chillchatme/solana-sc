from solana.rpc.api import Client as SolanaClient
from solana.rpc.commitment import Confirmed
from spl.token.instructions import get_associated_token_address


class Client:
    def __init__(self):
        self.client = SolanaClient("https://api.devnet.solana.com")

    def token_amount(self, account, mint):
        token_address = get_associated_token_address(account, mint)
        answer = self.client.get_token_account_balance(
            token_address, Confirmed)
        return answer['result']['value']['uiAmount']

    def token_account_exists(self, account, mint):
        token_address = get_associated_token_address(account, mint)
        answer = self.client.get_token_account_balance(
            token_address, Confirmed)
        return 'result' in answer
