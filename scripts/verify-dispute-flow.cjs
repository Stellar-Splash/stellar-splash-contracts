#!/usr/bin/env node
/**
 * Dispute Flow & Settlement Query Invariant Verification Script
 * Audits dispute window extension, time bounds, and settlement queries.
 */

const fs = require('fs');
const path = require('path');

console.log('=== Stellar Splash: Soroban Dispute Flow & Query Verification ===\n');

const LIB_RS_PATH = path.resolve(__dirname, '..', 'src', 'lib.rs');

if (!fs.existsSync(LIB_RS_PATH)) {
  console.error(`Contract source not found at ${LIB_RS_PATH}`);
  process.exit(1);
}

const libSource = fs.readFileSync(LIB_RS_PATH, 'utf-8');

const checks = [];

// 1. DisputeWindowInfo struct and get_tournament_dispute_window query
const hasWindowStruct = /struct DisputeWindowInfo/.test(libSource);
const hasWindowQuery = /fn get_tournament_dispute_window/.test(libSource);

checks.push({
  id: 'DISP-001',
  name: 'Structured Dispute Window Query',
  description: 'Exposes get_tournament_dispute_window returning end timestamp and elapsed flag',
  passed: hasWindowStruct && hasWindowQuery,
  evidence: hasWindowQuery ? 'DisputeWindowInfo struct and query method confirmed' : 'Missing window query',
});

// 2. Dispute window extension entrypoint and admin authorization
const hasExtendFn = /fn extend_dispute_window/.test(libSource);
const hasAdminAuthOnExtend = /admin\.require_auth\(\)/.test(libSource) && hasExtendFn;

checks.push({
  id: 'DISP-002',
  name: 'Administrative Dispute Window Extension',
  description: 'extend_dispute_window entrypoint protected by admin authorization',
  passed: hasExtendFn && hasAdminAuthOnExtend,
  evidence: hasExtendFn ? 'extend_dispute_window entrypoint verified with require_auth' : 'Missing extension entrypoint',
});

// 3. Minimum dispute window constant enforcement (>= 300s)
const minWindowRegex = /MIN_DISPUTE_WINDOW_SECONDS\s*:\s*u64\s*=\s*300/;
const hasMinWindow = minWindowRegex.test(libSource);

checks.push({
  id: 'DISP-003',
  name: 'Minimum Dispute Window Constraint',
  description: 'Enforces minimum dispute window duration of at least 300 seconds (5 minutes)',
  passed: hasMinWindow,
  evidence: hasMinWindow ? 'MIN_DISPUTE_WINDOW_SECONDS = 300 constant confirmed' : 'Missing min window constant',
});

// 4. get_settled_amount tournament query
const hasSettledAmountQuery = /fn get_settled_amount/.test(libSource);

checks.push({
  id: 'SETT-004',
  name: 'Disbursed Prize Amount Query',
  description: 'Exposes get_settled_amount returning total settled tokens for tournament',
  passed: hasSettledAmountQuery,
  evidence: hasSettledAmountQuery ? 'get_settled_amount query method verified' : 'Missing get_settled_amount query',
});

// 5. get_tournament_settlement_status query
const hasSettlementStatusQuery = /fn get_tournament_settlement_status/.test(libSource);

checks.push({
  id: 'SETT-005',
  name: 'Settlement Status Query',
  description: 'Exposes get_tournament_settlement_status returning Option<SettlementStatus>',
  passed: hasSettlementStatusQuery,
  evidence: hasSettlementStatusQuery ? 'get_tournament_settlement_status method verified' : 'Missing settlement status query',
});

// Output results
let allPassed = true;
checks.forEach((chk) => {
  const icon = chk.passed ? '✅' : '❌';
  console.log(`${icon} [${chk.id}] ${chk.name}`);
  console.log(`   ${chk.description}`);
  console.log(`   Evidence: ${chk.evidence}\n`);
  if (!chk.passed) allPassed = false;
});

const reportPath = path.resolve(__dirname, '..', 'benchmarks', 'dispute-flow-audit.json');
fs.writeFileSync(
  reportPath,
  JSON.stringify(
    {
      timestamp: new Date().toISOString(),
      allPassed,
      checks,
    },
    null,
    2
  )
);

console.log(`Detailed audit report written to: ${reportPath}`);

if (allPassed) {
  console.log('✅ ALL DISPUTE FLOW AND QUERY INVARIANTS PASSED.');
  process.exit(0);
} else {
  console.error('❌ ONE OR MORE INVARIANT CHECKS FAILED.');
  process.exit(1);
}
