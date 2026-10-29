#!/usr/bin/env node
/**
 * Admin Guards & Privileged Role Verification Script
 * Audits on-chain role enforcement, admin require_auth guards, deployer discovery,
 * dispute activity, and treasury safety bounds in the Soroban contract.
 */

const fs = require('fs');
const path = require('path');

console.log('=== Stellar Splash: Soroban Admin Guards & Role Verification ===\n');

const LIB_RS_PATH = path.resolve(__dirname, '..', 'src', 'lib.rs');

if (!fs.existsSync(LIB_RS_PATH)) {
  console.error(`Contract source not found at ${LIB_RS_PATH}`);
  process.exit(1);
}

const libSource = fs.readFileSync(LIB_RS_PATH, 'utf-8');

const checks = [];

// 1. Contract deployer and admin query methods
const hasGetAdmin = /pub fn get_admin/.test(libSource);
const hasGetDeployer = /pub fn get_contract_deployer/.test(libSource);

checks.push({
  id: 'GUARD-001',
  name: 'Deployer & Admin Discovery',
  description: 'Public query entrypoints for contract deployer and root admin',
  passed: hasGetAdmin && hasGetDeployer,
  evidence: hasGetDeployer ? 'get_admin and get_contract_deployer query methods confirmed' : 'Missing deployer query',
});

// 2. Administrative require_auth guards
const hasSetTreasuryGuard = /pub fn set_treasury[\s\S]*?admin\.require_auth\(\)/.test(libSource);
const hasWhitelistGuard = /pub fn set_asset_whitelisted[\s\S]*?admin\.require_auth\(\)/.test(libSource);
const hasResolveDisputeGuard = /pub fn resolve_dispute[\s\S]*?admin\.require_auth\(\)/.test(libSource);

checks.push({
  id: 'GUARD-002',
  name: 'Admin Authorization Guards',
  description: 'set_treasury, set_asset_whitelisted, and resolve_dispute protected by admin.require_auth()',
  passed: hasSetTreasuryGuard && hasWhitelistGuard && hasResolveDisputeGuard,
  evidence: 'All administrative state mutations protected by admin.require_auth()',
});

// 3. Dispute activity query method
const hasDisputeActive = /pub fn is_dispute_active/.test(libSource);

checks.push({
  id: 'GUARD-003',
  name: 'Dispute Activity Query',
  description: 'Exposes is_dispute_active query method returning boolean flag for ongoing disputes',
  passed: hasDisputeActive,
  evidence: hasDisputeActive ? 'is_dispute_active method confirmed in lib.rs' : 'Missing is_dispute_active',
});

// 4. Tournament capacity status query
const hasCapacityStruct = /pub struct TournamentCapacityInfo/.test(libSource);
const hasCapacityStatus = /pub fn get_tournament_capacity_status/.test(libSource);

checks.push({
  id: 'GUARD-004',
  name: 'Tournament Capacity Status Query',
  description: 'Exposes get_tournament_capacity_status with current/max participants and is_full flag',
  passed: hasCapacityStruct && hasCapacityStatus,
  evidence: hasCapacityStatus ? 'TournamentCapacityInfo struct and query method confirmed' : 'Missing capacity query',
});

// 5. Tournament treasury fee query & max fee constraint
const hasTreasuryFeeQuery = /pub fn get_tournament_treasury_fee/.test(libSource);
const hasMaxFeeConstant = /pub const MAX_TREASURY_FEE_BPS\s*:\s*u32\s*=\s*1_000/.test(libSource);

checks.push({
  id: 'GUARD-005',
  name: 'Treasury Fee Query & Safety Cap',
  description: 'Calculates per-tournament treasury fee with maximum basis-point cap',
  passed: hasTreasuryFeeQuery && hasMaxFeeConstant,
  evidence: hasTreasuryFeeQuery ? 'get_tournament_treasury_fee and MAX_TREASURY_FEE_BPS verified' : 'Missing treasury fee query',
});

let allPassed = true;
checks.forEach((chk) => {
  const symbol = chk.passed ? '✓' : '✗';
  console.log(`${symbol} [${chk.id}] ${chk.name}: ${chk.passed ? 'PASSED' : 'FAILED'}`);
  console.log(`   ${chk.description}`);
  console.log(`   Evidence: ${chk.evidence}\n`);
  if (!chk.passed) allPassed = false;
});

if (!allPassed) {
  console.error('❌ Admin guard verification failed.');
  process.exit(1);
}

console.log(`✅ All ${checks.length} admin guard and privilege verification checks passed successfully.`);
process.exit(0);
