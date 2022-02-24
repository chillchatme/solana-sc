from pathlib import Path
import os
import random
import shutil
import subprocess
import sys
import unittest
import uuid

from utils import *
from client import Client


def runCli(args: str):
    print(f'{EXECUTABLE_NAME} {args}')

    cmd = [str(EXECUTABLE_PATH)] + args.split()
    output = subprocess.run(cmd, check=False, capture_output=True)

    stdout = output.stdout.decode('UTF-8')
    returncode = output.returncode

    if returncode == 0:
        print(stdout)

    return stdout, returncode


class CliTest(unittest.TestCase):
    def setUp(self):
        prefix = str(uuid.uuid4())[:8]
        self.client = Client()
        self.keyfile = None
        self.save_path = PROJECT_DIR / f'{prefix}.mint.pubkey'

        if self.save_path.exists():
            print(f'Move or delete {self.save_path}')
            sys.exit(1)

        if DEFAULT_MINT_PATH.exists():
            print(f'Move or delete {DEFAULT_MINT_PATH}')
            sys.exit(1)

        if DEFAULT_KEY_PATH.exists():
            new_keypair_filename = f'{prefix}.id.json'
            self.keyfile = DEFAULT_KEY_PATH.parent / new_keypair_filename

            if self.keyfile.exists():
                sys.exit(1)

            shutil.move(DEFAULT_KEY_PATH, self.keyfile)

    def tearDown(self):
        if DEFAULT_MINT_PATH.is_file():
            os.remove(DEFAULT_MINT_PATH)

        if self.save_path.is_file():
            os.remove(self.save_path)

        if self.keyfile is not None:
            shutil.move(self.keyfile, DEFAULT_KEY_PATH)

    def test_initial_mint(self):
        balance = 0
        for _ in range(3):
            amount = random.randint(0, 1000)
            balance += amount
            output, code = runCli(f'mint {amount}')
            self.assertEqual(code, 0)
            self.assertTrue(str(balance) in output)
            self.assertTrue(DEFAULT_KEY_PATH.is_file())
            self.assertTrue(DEFAULT_MINT_PATH.is_file())

        output, code = runCli('balance')
        self.assertTrue(str(balance) in output)
        self.assertEqual(code, 0)

        owner = default_owner()
        mint = default_mintfile()
        amount = self.client.token_amount(owner, mint)
        self.assertEqual(amount, balance)

    def test_transfer(self):
        initial_balance = 1000
        balance = initial_balance
        output, code = runCli(f'mint {balance}')

        owner = default_owner()
        receiver = receiver_pubkey()
        mint = default_mintfile()

        amount = self.client.token_amount(owner, mint)
        self.assertEqual(amount, balance)
        self.assertEqual(code, 0)
        self.assertFalse(self.client.token_account_exists(receiver, mint))
        self.assertTrue(str(balance) in output)

        for _ in range(3):
            if balance == 0:
                break

            amount = random.randint(0, balance)
            if amount % 2:
                _, code = runCli(f'transfer {receiver} {amount}')
            else:
                _, code = runCli(f'transfer {RECEIVER_PATH} {amount}')

            balance -= amount
            self.assertTrue(self.client.token_account_exists(receiver, mint))
            self.assertEqual(code, 0)

        output, _ = runCli('balance')
        owner_amount = self.client.token_amount(owner, mint)
        self.assertEqual(owner_amount, balance)
        self.assertTrue(str(balance) in output)

        output, _ = runCli(f'balance --owner {owner}')
        owner_amount = self.client.token_amount(owner, mint)
        self.assertEqual(owner_amount, balance)
        self.assertTrue(str(balance) in output)

        output, _ = runCli(f'balance --owner {receiver}')
        receiver_amount = self.client.token_amount(receiver, mint)
        self.assertEqual(receiver_amount, initial_balance - balance)
        self.assertTrue(str(initial_balance - balance) in output)

        _, code = runCli(f'tranfser {receiver} 0')
        self.assertNotEqual(code, 0)


if __name__ == '__main__':
    unittest.main()
