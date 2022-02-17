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
        recipient = recipient_pubkey()
        mint = default_mintfile()

        amount = self.client.token_amount(owner, mint)
        self.assertEqual(amount, balance)
        self.assertEqual(code, 0)
        self.assertFalse(self.client.token_account_exists(recipient, mint))
        self.assertTrue(str(balance) in output)

        for _ in range(3):
            if balance == 0:
                break

            amount = random.randint(0, balance)
            if amount % 2:
                _, code = runCli(f'transfer {recipient} {amount}')
            else:
                _, code = runCli(f'transfer {RECIPIENT_PATH} {amount}')

            balance -= amount
            self.assertTrue(self.client.token_account_exists(recipient, mint))
            self.assertEqual(code, 0)

        output, _ = runCli('balance')
        owner_amount = self.client.token_amount(owner, mint)
        self.assertEqual(owner_amount, balance)
        self.assertTrue(str(balance) in output)

        output, _ = runCli(f'balance --owner {owner}')
        owner_amount = self.client.token_amount(owner, mint)
        self.assertEqual(owner_amount, balance)
        self.assertTrue(str(balance) in output)

        output, _ = runCli(f'balance --owner {recipient}')
        receiver_amount = self.client.token_amount(recipient, mint)
        self.assertEqual(receiver_amount, initial_balance - balance)
        self.assertTrue(str(initial_balance - balance) in output)

        _, code = runCli(f'tranfser {recipient} 0')
        self.assertNotEqual(code, 0)

    def test_initialization(self):
        initial_balance = 1000
        balance = initial_balance
        runCli(f'mint {balance}')

        owner = default_owner()
        mint = default_mintfile()

        total_mint_share = 100
        total_transaction_share = 100

        r_1 = Keypair.generate().public_key
        m_1 = random.randint(0, total_mint_share)
        t_1 = random.randint(0, total_transaction_share)

        total_mint_share -= m_1
        total_transaction_share -= t_1

        r_2 = Keypair.generate().public_key
        m_2 = random.randint(0, total_mint_share)
        t_2 = random.randint(0, total_transaction_share)

        total_mint_share -= m_2
        total_transaction_share -= t_2

        r_3 = Keypair.generate().public_key
        m_3 = total_mint_share
        t_3 = total_transaction_share

        character = random.random() + random.randint(0, 100)
        pet = random.random() + random.randint(0, 100)
        emote = random.random() + random.randint(0, 100)
        tileset = random.random() + random.randint(0, 100)
        item = random.random() + random.randint(0, 100)

        args = '\n\t'.join(("initialize",
                            f"--character {character}",
                            f"--emote {emote}",
                            f"--item {item}",
                            f"--pet {pet}",
                            f"--tileset {tileset}",
                            f"--recipient {r_1}",
                            f"--mint-share {m_1}",
                            f"--transaction-share {t_1}",
                            f"--recipient {r_2}",
                            f"--mint-share {m_2}",
                            f"--transaction-share {t_2}",
                            f"--recipient {r_3}",
                            f"--mint-share {m_3}",
                            f"--transaction-share {t_3}"
                            ))

        output, code = runCli(args)
        self.assertEqual(code, 0)


if __name__ == '__main__':
    unittest.main()
