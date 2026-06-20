#!/usr/bin/env node
// Zagros Testnet: Reduce validator count from 21 to 4 via sudo
// Uses @pezkuwi/api (ESM)

import { ApiPromise, WsProvider } from '/home/mamostehp/pezkuwi-api/node_modules/@pezkuwi/api/build/index.js';
import { Keyring } from '/home/mamostehp/pezkuwi-api/node_modules/@pezkuwi/keyring/build/cjs/index.js';
import { cryptoWaitReady } from '/home/mamostehp/pezkuwi-api/node_modules/@pezkuwi/util-crypto/build/cjs/index.js';

const ZAGROS_RPC = 'ws://217.77.6.126:9948';
const SUDO_SEED = process.env.SUDO_MNEMONIC || '******';
const NEW_VALIDATOR_COUNT = 4;

async function main() {
  console.log('=== ZAGROS VALIDATOR COUNT REDUCTION ===');
  console.log(`Target: ${NEW_VALIDATOR_COUNT} validators`);
  console.log(`RPC: ${ZAGROS_RPC}`);
  console.log();

  // Wait for crypto
  await cryptoWaitReady();

  // Create keyring and add sudo account
  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const sudo = keyring.addFromUri(SUDO_SEED);
  console.log(`Sudo account: ${sudo.address}`);

  // Connect to Zagros
  const provider = new WsProvider(ZAGROS_RPC);
  const api = await ApiPromise.create({
    provider,
    signedExtensions: {
      AuthorizeCall: {
        extrinsic: {},
        payload: {}
      }
    }
  });

  console.log(`Connected to: ${(await api.rpc.system.chain()).toString()}`);
  const version = await api.rpc.state.getRuntimeVersion();
  console.log(`Runtime version: ${version.specVersion.toString()}`);

  // Check current sudo key
  const sudoKey = await api.query.sudo.key();
  console.log(`On-chain sudo key: ${sudoKey.toString()}`);
  console.log(`Our key matches: ${sudoKey.toString() === sudo.address}`);
  console.log();

  // Check current validator count
  const currentCount = await api.query.staking.validatorCount();
  console.log(`Current validator count: ${currentCount.toString()}`);

  // Check current era
  const currentEra = await api.query.staking.currentEra();
  console.log(`Current era: ${currentEra.toString()}`);
  console.log();

  // Step 1: Set validator count to 4
  console.log(`[1/2] Setting validator count to ${NEW_VALIDATOR_COUNT}...`);
  const setValidatorCountCall = api.tx.staking.setValidatorCount(NEW_VALIDATOR_COUNT);
  const sudoCall1 = api.tx.sudo.sudo(setValidatorCountCall);

  try {
    const result1 = await new Promise((resolve, reject) => {
      sudoCall1.signAndSend(sudo, { nonce: -1 }, ({ status, events, dispatchError }) => {
        if (dispatchError) {
          if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            reject(new Error(`${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`));
          } else {
            reject(new Error(dispatchError.toString()));
          }
        }
        if (status.isInBlock) {
          console.log(`  Included in block: ${status.asInBlock.toString()}`);
          // Check for Sudid event
          const sudidEvent = events.find(({ event }) =>
            event.section === 'sudo' && event.method === 'Sudid'
          );
          if (sudidEvent) {
            const result = sudidEvent.event.data[0];
            if (result.isOk) {
              console.log('  Sudo executed successfully!');
            } else {
              console.log(`  Sudo dispatch error: ${result.asErr.toString()}`);
            }
          }
          resolve(status.asInBlock.toString());
        }
      });
    });
  } catch (e) {
    console.error(`  ERROR: ${e.message}`);
    await api.disconnect();
    process.exit(1);
  }

  // Verify
  const newCount = await api.query.staking.validatorCount();
  console.log(`  Validator count now: ${newCount.toString()}`);
  console.log();

  // Step 2: Force new era
  console.log('[2/2] Forcing new era...');
  const forceNewEraCall = api.tx.staking.forceNewEra();
  const sudoCall2 = api.tx.sudo.sudo(forceNewEraCall);

  try {
    const result2 = await new Promise((resolve, reject) => {
      sudoCall2.signAndSend(sudo, { nonce: -1 }, ({ status, events, dispatchError }) => {
        if (dispatchError) {
          if (dispatchError.isModule) {
            const decoded = api.registry.findMetaError(dispatchError.asModule);
            reject(new Error(`${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`));
          } else {
            reject(new Error(dispatchError.toString()));
          }
        }
        if (status.isInBlock) {
          console.log(`  Included in block: ${status.asInBlock.toString()}`);
          const sudidEvent = events.find(({ event }) =>
            event.section === 'sudo' && event.method === 'Sudid'
          );
          if (sudidEvent) {
            const result = sudidEvent.event.data[0];
            if (result.isOk) {
              console.log('  Sudo executed successfully!');
            } else {
              console.log(`  Sudo dispatch error: ${result.asErr.toString()}`);
            }
          }
          resolve(status.asInBlock.toString());
        }
      });
    });
  } catch (e) {
    console.error(`  ERROR: ${e.message}`);
    await api.disconnect();
    process.exit(1);
  }

  // Check forceEra storage
  const forceEra = await api.query.staking.forceEra();
  console.log(`  ForceEra: ${forceEra.toString()}`);
  console.log();

  console.log('=== DONE ===');
  console.log(`Validator count set to ${NEW_VALIDATOR_COUNT}`);
  console.log('ForceNewEra triggered - new era will start at next session boundary');
  console.log('GRANDPA should start finalizing once new authority set (4 validators) takes effect');

  await api.disconnect();
  process.exit(0);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
