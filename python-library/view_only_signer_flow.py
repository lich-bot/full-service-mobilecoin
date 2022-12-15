import asyncio
import subprocess
import sys
from fullservice import FullServiceAPIv2 as v2
import pytest
import json
import os

repo_root_dir = (
    subprocess.check_output("git rev-parse --show-toplevel", shell=True)
    .decode("utf8")
    .strip()
)
sys.path.append(
    "{}/.internal-ci/test/fs-integration".format(repo_root_dir)
)  # we're importing the basic.py file as the integration test framework

from basic import TestUtils as Utils

fs = v2()


@pytest.mark.asyncio
async def test_view_only_transaction(amount_pmob: int = 600000000):
    utils = Utils()
    Utils.get_mnemonics() # this breaks if called from the instance of the class..    
    alice = await utils.init_test_accounts(0, "alice", True)
    alice_export = await fs.export_account_secrets(alice.id)
    entropy = (
        alice_export.get("result")
        .get("account_secrets")
        .get("mnemonic")
        .removeprefix("(")
        .removesuffix(")")
    )  # clean up entropy response
    print(entropy)
    os.system(f"../target/release/transaction-signer import '{entropy}' -n alice") 
    # set up the acount as usual, but need to remove it before importing as a view only account. 
    os.system(f"../target/release/transaction-signer view-only-import-package mobilecoin_secret_mnemonic_{alice.id[0:6]}.json")
    await utils.clean()
    request: dict = json.loads(open(f"mobilecoin_view_account_import_package_{alice.id[0:6]}.json", "r").read()).get("params")
    await fs.import_view_only_account(**request)

    balance_before = int(
        (await fs.get_account_status(alice.id))
        .get("result")
        .get("balance_per_token")
        .get("0")
        .get("unspent")
    )

    unsigned_tx_request = await fs.build_unsigned_transaction(
        alice.id,
        amount={"value": str(amount_pmob), "token_id": str(0)},
        recipient_public_address=bob.main_address,
    )

    bob = await utils.init_test_accounts(1, "bob", True)

    # write the unsigned transaction request to a file
    to_json = json.dumps(unsigned_tx_request.get("result"), indent=4)
    with open("transaction_request.json", "w") as outfile:
        outfile.write(to_json)

    # get id for mnemonic file name and sign transaction
    id = alice.id[0:6]
    os.system(
        f"../target/release/transaction-signer sign mobilecoin_secret_mnemonic_{id}.json transaction_request.json"
    )

    with open("transaction_request.json_completed.json", "r") as infile:
        signed_tx: dict = json.load(infile)
    tx_proposal = signed_tx.get("params").get("tx_proposal")
    await fs.submit_transaction(tx_proposal)

    await utils.wait_two_blocks()
    balance_after = int(
        (await fs.get_account_status(alice.id))
        .get("result")
        .get("balance_per_token")
        .get("0")
        .get("unspent")
    )
    print(balance_before, balance_after)
    assert balance_before > balance_after, "transaction failed"
    exit


asyncio.run(test_view_only_transaction())
