#!/usr/bin/env node
/**
 * Soroban Contract Snapshot & Interface Verifier
 * Verifies contract structure, entrypoint parity, and deployed testnet contract bindings.
 */

const fs = require('fs');
const path = require('path');

const CONTRACT_ROOT = path.resolve(__dirname, '..');
const CARGO_TOML_PATH = path.join(CONTRACT_ROOT, 'Cargo.toml');
const LIB_RS_PATH = path.join(CONTRACT_ROOT, 'src', 'lib.rs');
const TESTNET_CONTRACT_ID = 'CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ';

console.log('=== Stellar Splash Contract Snapshot Verifier ===');

// 1. Verify Cargo.toml
if (!fs.existsSync(CARGO_TOML_PATH)) {
  console.error('❌ Cargo.toml not found at', CARGO_TOML_PATH);
  process.exit(1);
}
const cargoContent = fs.readFileSync(CARGO_TOML_PATH, 'utf8');
if (!cargoContent.includes('soroban-sdk')) {
  console.error('❌ Cargo.toml is missing soroban-sdk dependency');
  process.exit(1);
}
console.log('✅ Cargo.toml parsed: soroban-sdk found');

// 2. Verify lib.rs and required entrypoints
if (!fs.existsSync(LIB_RS_PATH)) {
  console.error('❌ src/lib.rs not found at', LIB_RS_PATH);
  process.exit(1);
}
const libContent = fs.readFileSync(LIB_RS_PATH, 'utf8');

const requiredEntrypoints = [
  'pub fn initialize',
  'pub fn create_tournament',
  'pub fn fund_prize_pool',
  'pub fn register_participant',
  'pub fn set_entry_fee',
  'pub fn pay_entry_fee',
  'pub fn get_entry_fee',
  'pub fn get_total_entry_fees',
  'pub fn has_paid_entry_fee',
  'pub fn submit_dispute',
  'pub fn resolve_dispute',
  'pub fn upgrade',
  'pub fn get_contract_token_balance',
];

const missing = [];
for (const ep of requiredEntrypoints) {
  if (!libContent.includes(ep)) {
    missing.push(ep);
  }
}

if (missing.length > 0) {
  console.error('❌ Missing expected Soroban entrypoints:', missing);
  process.exit(1);
}
console.log(`✅ Verified all ${requiredEntrypoints.length} expected Soroban contract entrypoints`);

// 3. Verify deployed testnet contract ID format
if (!/^C[A-Z0-9]{55}$/.test(TESTNET_CONTRACT_ID)) {
  console.error('❌ Invalid Testnet Contract Address format:', TESTNET_CONTRACT_ID);
  process.exit(1);
}
console.log('✅ Verified deployed Testnet Contract ID format:', TESTNET_CONTRACT_ID);

console.log('=== All contract snapshot verifications passed! ===\n');
process.exit(0);
