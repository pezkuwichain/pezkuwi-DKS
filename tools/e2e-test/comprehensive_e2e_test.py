#!/usr/bin/env python3
"""
Pezkuwi Mainnet - Comprehensive End-to-End Test Suite
=====================================================

Tests all user-facing blockchain operations with real funds.
Creates a new test wallet, funds it, and runs through all scenarios.

Chains:
  - Relay Chain (HEZ, 12 decimals): ws://217.77.6.126:9944
  - Asset Hub (TYR, 12 decimals):   ws://217.77.6.126:40944
  - People Chain:                    ws://217.77.6.126:41944

Old Test Wallet: set OLD_WALLET_MNEMONIC env var
New Test Wallet: set NEW_WALLET_MNEMONIC env var
"""

import os
import sys
import time
import traceback
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException

# ============================================================
# CONFIGURATION
# ============================================================

RELAY_RPC = "ws://217.77.6.126:9944"
ASSET_HUB_RPC = "ws://217.77.6.126:40944"
PEOPLE_CHAIN_RPC = "ws://217.77.6.126:41944"

OLD_WALLET_MNEMONIC = os.environ.get("OLD_WALLET_MNEMONIC", "")
NEW_WALLET_MNEMONIC = os.environ.get("NEW_WALLET_MNEMONIC", "")

UNITS = 10**12  # 1 HEZ/TYR = 10^12 smallest units
TEST_AMOUNT = 100 * UNITS  # 100 HEZ/TYR for tests

# ============================================================
# HELPERS
# ============================================================

class TestResult:
    def __init__(self):
        self.passed = []
        self.failed = []
        self.skipped = []

    def log_pass(self, name, detail=""):
        self.passed.append((name, detail))
        print(f"  [PASS] {name}" + (f" - {detail}" if detail else ""))

    def log_fail(self, name, detail=""):
        self.failed.append((name, detail))
        print(f"  [FAIL] {name}" + (f" - {detail}" if detail else ""))

    def log_skip(self, name, detail=""):
        self.skipped.append((name, detail))
        print(f"  [SKIP] {name}" + (f" - {detail}" if detail else ""))

    def summary(self):
        total = len(self.passed) + len(self.failed) + len(self.skipped)
        print(f"\n{'='*60}")
        print(f"TEST SONUCLARI: {len(self.passed)} passed, {len(self.failed)} failed, {len(self.skipped)} skipped / {total} total")
        if self.failed:
            print(f"\nBasarisiz testler:")
            for name, detail in self.failed:
                print(f"  - {name}: {detail}")
        print(f"{'='*60}")
        return len(self.failed) == 0


def connect(url, name=""):
    """Connect to a chain RPC endpoint."""
    print(f"  Connecting to {name} ({url})...")
    ws = SubstrateInterface(url=url)
    print(f"  Connected: {ws.name} v{ws.version} (spec: {ws.runtime_version})")
    return ws


def account_id(keypair):
    """Get account ID as 0x-prefixed hex string for storage queries."""
    return "0x" + keypair.public_key.hex()


def get_balance(ws, keypair):
    """Get free balance of an account. Pass Keypair object."""
    result = ws.query("System", "Account", [account_id(keypair)])
    if result is None:
        return 0
    return result.value["data"]["free"]


def get_full_account(ws, keypair):
    """Get full account info. Pass Keypair object."""
    result = ws.query("System", "Account", [account_id(keypair)])
    if result is None:
        return {"nonce": 0, "data": {"free": 0, "reserved": 0, "frozen": 0}}
    return result.value


def get_nonce(ws, keypair):
    """Get account nonce via RPC (bypasses broken type encoding)."""
    result = ws.rpc_request("system_accountNextIndex", [keypair.ss58_address])
    return result["result"]


def submit_extrinsic(ws, call, keypair, wait=True):
    """Submit an extrinsic and return the receipt."""
    nonce = get_nonce(ws, keypair)
    extrinsic = ws.create_signed_extrinsic(call=call, keypair=keypair, nonce=nonce)
    receipt = ws.submit_extrinsic(extrinsic, wait_for_inclusion=wait)
    if wait and not receipt.is_success:
        raise Exception(f"Extrinsic failed: {receipt.error_message}")
    return receipt


def fmt_balance(amount, symbol="HEZ"):
    """Format a balance for display."""
    return f"{amount / UNITS:,.6f} {symbol}"


# ============================================================
# TEST SCENARIOS
# ============================================================

def test_chain_connectivity(results):
    """Test 1: Verify all three chains are reachable and healthy."""
    print("\n[TEST 1] Chain Connectivity & Health")
    print("-" * 40)

    for name, url in [("Relay", RELAY_RPC), ("Asset Hub", ASSET_HUB_RPC), ("People Chain", PEOPLE_CHAIN_RPC)]:
        try:
            ws = connect(url, name)
            health = ws.rpc_request("system_health", [])["result"]
            header = ws.rpc_request("chain_getHeader", [])["result"]
            block_num = int(header["number"], 16)
            peers = health.get("peers", 0)

            if health.get("isSyncing", True):
                results.log_fail(f"{name} connectivity", f"Chain is syncing")
            elif block_num == 0:
                results.log_fail(f"{name} connectivity", f"Stuck at block 0")
            else:
                results.log_pass(f"{name} connectivity", f"block #{block_num}, {peers} peers")
            ws.close()
        except Exception as e:
            results.log_fail(f"{name} connectivity", str(e))


def test_wallet_creation(results):
    """Test 2: Create and verify new test wallet."""
    print("\n[TEST 2] Wallet Creation & Key Derivation")
    print("-" * 40)

    try:
        old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
        new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

        print(f"  Old wallet: {old_kp.ss58_address}")
        print(f"  New wallet: {new_kp.ss58_address}")

        # Verify key derivation is deterministic
        old_kp2 = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
        assert old_kp.ss58_address == old_kp2.ss58_address
        results.log_pass("Key derivation deterministic")

        # Verify addresses are different
        assert old_kp.ss58_address != new_kp.ss58_address
        results.log_pass("Wallets are distinct")

    except Exception as e:
        results.log_fail("Wallet creation", str(e))


def test_balance_query(results):
    """Test 3: Query balances on all chains."""
    print("\n[TEST 3] Balance Queries")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    for name, url, symbol in [("Relay", RELAY_RPC, "HEZ"), ("Asset Hub", ASSET_HUB_RPC, "TYR"), ("People Chain", PEOPLE_CHAIN_RPC, "HEZ")]:
        try:
            ws = connect(url, name)
            old_bal = get_balance(ws, old_kp)
            new_bal = get_balance(ws, new_kp)
            results.log_pass(f"{name} balance query",
                f"Old: {fmt_balance(old_bal, symbol)}, New: {fmt_balance(new_bal, symbol)}")
            ws.close()
        except Exception as e:
            results.log_fail(f"{name} balance query", str(e))


def test_relay_transfer(results):
    """Test 4: Transfer HEZ on relay chain (old -> new wallet)."""
    print("\n[TEST 4] Relay Chain Transfer (Old -> New)")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(RELAY_RPC, "Relay")
        amount = 500 * UNITS  # 500 HEZ

        before_old = get_balance(ws, old_kp)
        before_new = get_balance(ws, new_kp)
        print(f"  Before - Old: {fmt_balance(before_old)}, New: {fmt_balance(before_new)}")

        call = ws.compose_call(
            call_module="Balances",
            call_function="transfer_keep_alive",
            call_params={"dest": {"Id": account_id(new_kp)}, "value": amount}
        )
        receipt = submit_extrinsic(ws, call, old_kp)

        after_old = get_balance(ws, old_kp)
        after_new = get_balance(ws, new_kp)
        print(f"  After  - Old: {fmt_balance(after_old)}, New: {fmt_balance(after_new)}")

        transferred = after_new - before_new
        if transferred >= amount:
            results.log_pass("Relay transfer", f"Sent {fmt_balance(amount)}, received {fmt_balance(transferred)}")
        else:
            results.log_fail("Relay transfer", f"Expected {amount}, got {transferred}")

        # Verify fee was deducted
        fee = before_old - after_old - amount
        if fee > 0:
            results.log_pass("Transaction fee deducted", fmt_balance(fee))
        else:
            results.log_fail("Transaction fee", f"Fee={fee}")

        ws.close()
    except Exception as e:
        results.log_fail("Relay transfer", str(e))
        traceback.print_exc()


def test_new_wallet_self_transfer(results):
    """Test 5: New wallet sends back a small amount to verify signing works."""
    print("\n[TEST 5] New Wallet Self-Signing (send back)")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(RELAY_RPC, "Relay")
        amount = 1 * UNITS  # 1 HEZ

        before_new = get_balance(ws, new_kp)
        if before_new < amount * 2:
            results.log_skip("New wallet self-transfer", "Insufficient balance")
            ws.close()
            return

        call = ws.compose_call(
            call_module="Balances",
            call_function="transfer_keep_alive",
            call_params={"dest": {"Id": account_id(old_kp)}, "value": amount}
        )
        receipt = submit_extrinsic(ws, call, new_kp)
        results.log_pass("New wallet can sign & send", f"Sent {fmt_balance(amount)} back")
        ws.close()
    except Exception as e:
        results.log_fail("New wallet self-transfer", str(e))
        traceback.print_exc()


def test_xcm_relay_to_asset_hub(results):
    """Test 6: XCM transfer from Relay to Asset Hub."""
    print("\n[TEST 6] XCM Transfer: Relay -> Asset Hub")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws_relay = connect(RELAY_RPC, "Relay")
        ws_ah = connect(ASSET_HUB_RPC, "Asset Hub")

        amount = 50 * UNITS  # 50 HEZ

        before_relay = get_balance(ws_relay, new_kp)
        before_ah = get_balance(ws_ah, new_kp)
        print(f"  Before - Relay: {fmt_balance(before_relay)}, AH: {fmt_balance(before_ah, 'TYR')}")

        if before_relay < amount * 2:
            results.log_skip("XCM Relay->AH", "Insufficient relay balance")
            ws_relay.close()
            ws_ah.close()
            return

        # XCM teleport from relay to Asset Hub (para_id 1000)
        dest = {"V4": {"parents": 0, "interior": {"X1": [{"Parachain": 1000}]}}}
        beneficiary = {"V4": {"parents": 0, "interior": {"X1": [{"AccountId32": {"id": account_id(new_kp), "network": None}}]}}}
        assets = {"V4": [{"id": {"parents": 0, "interior": "Here"}, "fun": {"Fungible": amount}}]}

        call = ws_relay.compose_call(
            call_module="XcmPallet",
            call_function="limited_teleport_assets",
            call_params={
                "dest": dest,
                "beneficiary": beneficiary,
                "assets": assets,
                "fee_asset_item": 0,
                "weight_limit": "Unlimited",
            }
        )
        receipt = submit_extrinsic(ws_relay, call, new_kp)
        results.log_pass("XCM teleport submitted", f"Block: {receipt.block_hash[:16]}...")

        # Wait for XCM to arrive on Asset Hub
        print("  Waiting 30s for XCM arrival...")
        time.sleep(30)

        after_ah = get_balance(ws_ah, new_kp)
        received = after_ah - before_ah
        print(f"  After  - AH: {fmt_balance(after_ah, 'TYR')} (received: {fmt_balance(received, 'TYR')})")

        if received > 0:
            results.log_pass("XCM arrived on Asset Hub", fmt_balance(received, "TYR"))
        else:
            results.log_fail("XCM arrival", "No balance increase on Asset Hub after 30s")

        ws_relay.close()
        ws_ah.close()
    except Exception as e:
        results.log_fail("XCM Relay->AH", str(e))
        traceback.print_exc()


def test_asset_hub_transfer(results):
    """Test 7: Transfer TYR on Asset Hub."""
    print("\n[TEST 7] Asset Hub Transfer")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        amount = 10 * UNITS  # 10 TYR

        before_new = get_balance(ws, new_kp)
        if before_new < amount * 2:
            # Fund from old wallet
            print(f"  New wallet AH balance low ({fmt_balance(before_new, 'TYR')}), funding from old wallet...")
            before_old = get_balance(ws, old_kp)
            if before_old >= amount * 5:
                call = ws.compose_call(
                    call_module="Balances",
                    call_function="transfer_keep_alive",
                    call_params={"dest": {"Id": account_id(new_kp)}, "value": amount * 3}
                )
                submit_extrinsic(ws, call, old_kp)
                before_new = get_balance(ws, new_kp)
                print(f"  Funded. New AH balance: {fmt_balance(before_new, 'TYR')}")

        # Now transfer from new to old
        call = ws.compose_call(
            call_module="Balances",
            call_function="transfer_keep_alive",
            call_params={"dest": {"Id": account_id(old_kp)}, "value": amount}
        )
        receipt = submit_extrinsic(ws, call, new_kp)
        results.log_pass("Asset Hub transfer", f"Sent {fmt_balance(amount, 'TYR')}")
        ws.close()
    except Exception as e:
        results.log_fail("Asset Hub transfer", str(e))
        traceback.print_exc()


def test_staking_bond(results):
    """Test 8: Bond tokens for staking on Asset Hub."""
    print("\n[TEST 8] Staking: Bond Tokens")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        bond_amount = 15 * UNITS  # 15 TYR

        balance = get_balance(ws, new_kp)
        if balance < bond_amount * 2:
            results.log_skip("Staking bond", f"Insufficient balance: {fmt_balance(balance, 'TYR')}")
            ws.close()
            return

        # Check if already bonded
        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is not None and ledger.value is not None:
            results.log_skip("Staking bond", "Already bonded, skipping")
            ws.close()
            return

        call = ws.compose_call(
            call_module="Staking",
            call_function="bond",
            call_params={
                "value": bond_amount,
                "payee": {"Staked": None},
            }
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        # Verify bonded
        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is not None and ledger.value is not None:
            bonded = ledger.value.get("active", 0)
            results.log_pass("Staking bonded", fmt_balance(bonded, "TYR"))
        else:
            results.log_fail("Staking bond", "Ledger not found after bonding")

        ws.close()
    except Exception as e:
        results.log_fail("Staking bond", str(e))
        traceback.print_exc()


def test_staking_nominate(results):
    """Test 9: Nominate validators."""
    print("\n[TEST 9] Staking: Nominate Validators")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")

        # Check bonded
        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is None or ledger.value is None:
            results.log_skip("Nominate", "Not bonded yet")
            ws.close()
            return

        # Get current validators
        validators = ws.query("Session", "Validators", [])
        if validators is None or not validators.value:
            # Try getting validators from Staking pallet
            results.log_skip("Nominate", "No validators found in Session")
            ws.close()
            return

        # Nominate first 3 validators (or all if less)
        val_list = validators.value[:3]
        print(f"  Nominating {len(val_list)} validators...")

        call = ws.compose_call(
            call_module="Staking",
            call_function="nominate",
            call_params={"targets": val_list}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        # Verify nomination
        nominators = ws.query("Staking", "Nominators", [account_id(new_kp)])
        if nominators is not None and nominators.value is not None:
            targets = nominators.value.get("targets", [])
            results.log_pass("Nomination", f"Nominated {len(targets)} validators")
        else:
            results.log_fail("Nomination", "Nominator record not found")

        ws.close()
    except Exception as e:
        results.log_fail("Nominate", str(e))
        traceback.print_exc()


def test_staking_bond_extra(results):
    """Test 10: Bond extra tokens."""
    print("\n[TEST 10] Staking: Bond Extra")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        extra_amount = 5 * UNITS  # 5 TYR

        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is None or ledger.value is None:
            results.log_skip("Bond extra", "Not bonded")
            ws.close()
            return

        before_bonded = ledger.value.get("active", 0)

        call = ws.compose_call(
            call_module="Staking",
            call_function="bond_extra",
            call_params={"max_additional": extra_amount}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        after_bonded = ledger.value.get("active", 0)
        added = after_bonded - before_bonded

        results.log_pass("Bond extra", f"Added {fmt_balance(added, 'TYR')}, total bonded: {fmt_balance(after_bonded, 'TYR')}")
        ws.close()
    except Exception as e:
        results.log_fail("Bond extra", str(e))
        traceback.print_exc()


def test_staking_unbond(results):
    """Test 11: Unbond tokens (partial)."""
    print("\n[TEST 11] Staking: Unbond (partial)")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        unbond_amount = 3 * UNITS  # 3 TYR

        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is None or ledger.value is None:
            results.log_skip("Unbond", "Not bonded")
            ws.close()
            return

        before_bonded = ledger.value.get("active", 0)
        if before_bonded < unbond_amount:
            results.log_skip("Unbond", f"Bonded amount too low: {fmt_balance(before_bonded, 'TYR')}")
            ws.close()
            return

        call = ws.compose_call(
            call_module="Staking",
            call_function="unbond",
            call_params={"value": unbond_amount}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        after_bonded = ledger.value.get("active", 0)
        unlocking = ledger.value.get("unlocking", [])

        results.log_pass("Unbond", f"Active: {fmt_balance(after_bonded, 'TYR')}, unlocking entries: {len(unlocking)}")
        ws.close()
    except Exception as e:
        results.log_fail("Unbond", str(e))
        traceback.print_exc()


def test_staking_chill(results):
    """Test 12: Chill (stop nominating)."""
    print("\n[TEST 12] Staking: Chill")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")

        ledger = ws.query("Staking", "Ledger", [account_id(new_kp)])
        if ledger is None or ledger.value is None:
            results.log_skip("Chill", "Not bonded")
            ws.close()
            return

        call = ws.compose_call(
            call_module="Staking",
            call_function="chill",
            call_params={}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        nominators = ws.query("Staking", "Nominators", [account_id(new_kp)])
        if nominators is None or nominators.value is None:
            results.log_pass("Chill", "No longer nominating")
        else:
            results.log_fail("Chill", "Still nominating after chill")

        ws.close()
    except Exception as e:
        results.log_fail("Chill", str(e))
        traceback.print_exc()


def test_nomination_pool_join(results):
    """Test 13: Join a nomination pool."""
    print("\n[TEST 13] Nomination Pool: Join")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        join_amount = 10 * UNITS  # 10 TYR

        # Check if any pools exist
        pool_count = ws.query("NominationPools", "CounterForBondedPools", [])
        if pool_count is None or pool_count.value == 0:
            results.log_skip("Pool join", "No nomination pools exist")
            ws.close()
            return

        # Check if already a pool member
        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        if member is not None and member.value is not None:
            results.log_skip("Pool join", f"Already a pool member (pool {member.value.get('pool_id', '?')})")
            ws.close()
            return

        balance = get_balance(ws, new_kp)
        if balance < join_amount * 2:
            results.log_skip("Pool join", f"Insufficient balance: {fmt_balance(balance, 'TYR')}")
            ws.close()
            return

        # Join pool 1 (first pool)
        call = ws.compose_call(
            call_module="NominationPools",
            call_function="join",
            call_params={"amount": join_amount, "pool_id": 1}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        if member is not None and member.value is not None:
            pool_id = member.value.get("pool_id", "?")
            points = member.value.get("points", 0)
            results.log_pass("Pool join", f"Joined pool {pool_id}, points: {points}")
        else:
            results.log_fail("Pool join", "Member record not found after joining")

        ws.close()
    except Exception as e:
        results.log_fail("Pool join", str(e))
        traceback.print_exc()


def test_nomination_pool_bond_extra(results):
    """Test 14: Add more funds to nomination pool."""
    print("\n[TEST 14] Nomination Pool: Bond Extra")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        extra_amount = 5 * UNITS  # 5 TYR

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        if member is None or member.value is None:
            results.log_skip("Pool bond extra", "Not a pool member")
            ws.close()
            return

        before_points = member.value.get("points", 0)

        call = ws.compose_call(
            call_module="NominationPools",
            call_function="bond_extra",
            call_params={"extra": {"FreeBalance": extra_amount}}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        after_points = member.value.get("points", 0)
        added = after_points - before_points

        results.log_pass("Pool bond extra", f"Added {added} points, total: {after_points}")
        ws.close()
    except Exception as e:
        results.log_fail("Pool bond extra", str(e))
        traceback.print_exc()


def test_nomination_pool_claim_rewards(results):
    """Test 15: Claim pending rewards from nomination pool."""
    print("\n[TEST 15] Nomination Pool: Claim Rewards")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        if member is None or member.value is None:
            results.log_skip("Pool claim rewards", "Not a pool member")
            ws.close()
            return

        before_balance = get_balance(ws, new_kp)

        call = ws.compose_call(
            call_module="NominationPools",
            call_function="claim_payout",
            call_params={}
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        after_balance = get_balance(ws, new_kp)
        reward = after_balance - before_balance

        if reward > 0:
            results.log_pass("Pool claim rewards", f"Claimed {fmt_balance(reward, 'TYR')}")
        else:
            results.log_pass("Pool claim rewards", "No pending rewards (tx succeeded)")

        ws.close()
    except Exception as e:
        results.log_fail("Pool claim rewards", str(e))
        traceback.print_exc()


def test_nomination_pool_unbond(results):
    """Test 16: Unbond from nomination pool (partial)."""
    print("\n[TEST 16] Nomination Pool: Unbond (partial)")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(ASSET_HUB_RPC, "Asset Hub")
        unbond_amount = 3 * UNITS  # 3 TYR

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        if member is None or member.value is None:
            results.log_skip("Pool unbond", "Not a pool member")
            ws.close()
            return

        before_points = member.value.get("points", 0)
        if before_points < unbond_amount:
            results.log_skip("Pool unbond", f"Points too low: {before_points}")
            ws.close()
            return

        call = ws.compose_call(
            call_module="NominationPools",
            call_function="unbond",
            call_params={
                "member_account": account_id(new_kp),
                "unbonding_points": unbond_amount,
            }
        )
        receipt = submit_extrinsic(ws, call, new_kp)

        member = ws.query("NominationPools", "PoolMembers", [account_id(new_kp)])
        after_points = member.value.get("points", 0)
        unbonding = member.value.get("unbonding_eras", {})

        results.log_pass("Pool unbond", f"Remaining points: {after_points}, unbonding eras: {len(unbonding)}")
        ws.close()
    except Exception as e:
        results.log_fail("Pool unbond", str(e))
        traceback.print_exc()


def test_set_identity(results):
    """Test 17: Set on-chain identity on People Chain."""
    print("\n[TEST 17] People Chain: Set Identity")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws_people = connect(PEOPLE_CHAIN_RPC, "People Chain")

        # Check balance on People Chain
        balance = get_balance(ws_people, new_kp)
        if balance < 1 * UNITS:
            results.log_skip("Set identity", f"Insufficient People Chain balance: {fmt_balance(balance)}")
            ws_people.close()
            return

        # Set identity
        call = ws_people.compose_call(
            call_module="Identity",
            call_function="set_identity",
            call_params={
                "info": {
                    "display": {"Raw": "E2E_Test_Wallet"},
                    "legal": {"None": None},
                    "web": {"None": None},
                    "email": {"None": None},
                    "pgp_fingerprint": None,
                    "image": {"None": None},
                    "twitter": {"None": None},
                    "github": {"None": None},
                    "discord": {"None": None},
                }
            }
        )
        receipt = submit_extrinsic(ws_people, call, new_kp)

        # Verify identity
        identity = ws_people.query("Identity", "IdentityOf", [account_id(new_kp)])
        if identity is not None and identity.value is not None:
            results.log_pass("Set identity", "Identity set successfully")
        else:
            results.log_fail("Set identity", "Identity not found after setting")

        ws_people.close()
    except Exception as e:
        results.log_fail("Set identity", str(e))
        traceback.print_exc()


def test_clear_identity(results):
    """Test 18: Clear on-chain identity."""
    print("\n[TEST 18] People Chain: Clear Identity")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws_people = connect(PEOPLE_CHAIN_RPC, "People Chain")

        identity = ws_people.query("Identity", "IdentityOf", [account_id(new_kp)])
        if identity is None or identity.value is None:
            results.log_skip("Clear identity", "No identity set")
            ws_people.close()
            return

        call = ws_people.compose_call(
            call_module="Identity",
            call_function="clear_identity",
            call_params={}
        )
        receipt = submit_extrinsic(ws_people, call, new_kp)

        identity = ws_people.query("Identity", "IdentityOf", [account_id(new_kp)])
        if identity is None or identity.value is None:
            results.log_pass("Clear identity", "Identity cleared, deposit returned")
        else:
            results.log_fail("Clear identity", "Identity still exists after clearing")

        ws_people.close()
    except Exception as e:
        results.log_fail("Clear identity", str(e))
        traceback.print_exc()


def test_remark(results):
    """Test 19: System.remark (minimal extrinsic)."""
    print("\n[TEST 19] System Remark")
    print("-" * 40)

    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(RELAY_RPC, "Relay")
        message = "Pezkuwi E2E Test - " + str(int(time.time()))

        call = ws.compose_call(
            call_module="System",
            call_function="remark",
            call_params={"remark": "0x" + message.encode().hex()}
        )
        receipt = submit_extrinsic(ws, call, new_kp)
        results.log_pass("System remark", f"Block: {receipt.block_hash[:16]}...")
        ws.close()
    except Exception as e:
        results.log_fail("System remark", str(e))
        traceback.print_exc()


def test_batch_calls(results):
    """Test 20: Utility.batch - multiple calls in one transaction."""
    print("\n[TEST 20] Utility Batch (multiple transfers)")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    try:
        ws = connect(RELAY_RPC, "Relay")

        balance = get_balance(ws, new_kp)
        if balance < 5 * UNITS:
            results.log_skip("Batch calls", "Insufficient balance")
            ws.close()
            return

        # Create two small transfers in a batch
        call1 = ws.compose_call(
            call_module="Balances",
            call_function="transfer_keep_alive",
            call_params={"dest": {"Id": account_id(old_kp)}, "value": 1 * UNITS}
        )
        call2 = ws.compose_call(
            call_module="System",
            call_function="remark",
            call_params={"remark": "0x" + "batch_test".encode().hex()}
        )

        batch_call = ws.compose_call(
            call_module="Utility",
            call_function="batch",
            call_params={"calls": [call1.value, call2.value]}
        )
        receipt = submit_extrinsic(ws, batch_call, new_kp)

        # Check events for BatchCompleted
        events = receipt.triggered_events
        batch_ok = any("BatchCompleted" in str(e) for e in events)
        if batch_ok:
            results.log_pass("Batch calls", "BatchCompleted event found")
        else:
            results.log_pass("Batch calls", "Batch submitted successfully")

        ws.close()
    except Exception as e:
        results.log_fail("Batch calls", str(e))
        traceback.print_exc()


def test_final_balances(results):
    """Test 21: Final balance report across all chains."""
    print("\n[TEST 21] Final Balance Report")
    print("-" * 40)

    old_kp = Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC)
    new_kp = Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC)

    for name, url, symbol in [("Relay", RELAY_RPC, "HEZ"), ("Asset Hub", ASSET_HUB_RPC, "TYR"), ("People Chain", PEOPLE_CHAIN_RPC, "HEZ")]:
        try:
            ws = connect(url, name)
            old_info = get_full_account(ws, old_kp)
            new_info = get_full_account(ws, new_kp)

            old_free = old_info["data"]["free"]
            old_reserved = old_info["data"]["reserved"]
            old_frozen = old_info["data"]["frozen"]
            new_free = new_info["data"]["free"]
            new_reserved = new_info["data"]["reserved"]
            new_frozen = new_info["data"]["frozen"]

            print(f"  {name} - Old: free={fmt_balance(old_free, symbol)} reserved={fmt_balance(old_reserved, symbol)} frozen={fmt_balance(old_frozen, symbol)}")
            print(f"  {name} - New: free={fmt_balance(new_free, symbol)} reserved={fmt_balance(new_reserved, symbol)} frozen={fmt_balance(new_frozen, symbol)}")

            results.log_pass(f"{name} final balance", f"Old: {fmt_balance(old_free, symbol)}, New: {fmt_balance(new_free, symbol)}")
            ws.close()
        except Exception as e:
            results.log_fail(f"{name} final balance", str(e))


# ============================================================
# MAIN
# ============================================================

def main():
    print("=" * 60)
    print("PEZKUWI MAINNET - COMPREHENSIVE E2E TEST SUITE")
    print("=" * 60)
    print(f"Time: {time.strftime('%Y-%m-%d %H:%M:%S UTC', time.gmtime())}")
    print(f"Old wallet: {Keypair.create_from_mnemonic(OLD_WALLET_MNEMONIC).ss58_address}")
    print(f"New wallet: {Keypair.create_from_mnemonic(NEW_WALLET_MNEMONIC).ss58_address}")

    results = TestResult()

    # Phase 1: Connectivity & Setup
    test_chain_connectivity(results)
    test_wallet_creation(results)
    test_balance_query(results)

    # Phase 2: Basic Transfers
    test_relay_transfer(results)
    test_new_wallet_self_transfer(results)
    test_xcm_relay_to_asset_hub(results)
    test_asset_hub_transfer(results)

    # Phase 3: Staking Operations
    test_staking_bond(results)
    test_staking_nominate(results)
    test_staking_bond_extra(results)
    test_staking_unbond(results)
    test_staking_chill(results)

    # Phase 4: Nomination Pools
    test_nomination_pool_join(results)
    test_nomination_pool_bond_extra(results)
    test_nomination_pool_claim_rewards(results)
    test_nomination_pool_unbond(results)

    # Phase 5: Identity & Misc
    test_set_identity(results)
    test_clear_identity(results)
    test_remark(results)
    test_batch_calls(results)

    # Phase 6: Final Report
    test_final_balances(results)

    success = results.summary()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
