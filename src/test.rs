#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _, token::Client as TokenClient, token::StellarAssetClient, Address,
    BytesN, Env, String,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &contract_address.address()),
        StellarAssetClient::new(e, &contract_address.address()),
    )
}

#[test]
fn test_tournament_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.get_admin(), admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let prize_amount: i128 = 100_000_000;
    token_asset.mint(&funder, &prize_amount);

    let tournament_id: u64 = 101;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Splash Rush Championship #001"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 10,
        start_time: 1700000000,
        end_time: 1700086400,
    };

    let tourn = client.create_tournament(&creator, &config);
    assert_eq!(tourn.state, TournamentState::Funding);

    // Fund and open
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    client.open_tournament(&creator, &tournament_id);

    // Register players
    client.register_participant(&player1, &tournament_id);
    client.register_participant(&player2, &tournament_id);

    client.start_tournament(&creator, &tournament_id);
    client.complete_tournament(&creator, &tournament_id);

    assert_eq!(client.get_vault_balance(&tournament_id), prize_amount);
}

#[test]
fn test_verified_result_and_ranking_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let verifier = Address::generate(&env);
    let player1 = Address::generate(&env);

    let prize_amount: i128 = 50_000_000;
    token_asset.mint(&funder, &prize_amount);

    let tournament_id: u64 = 401;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Attestation Arena"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 8,
        start_time: 1000,
        end_time: 2000,
    };
    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    client.open_tournament(&creator, &tournament_id);
    client.register_participant(&player1, &tournament_id);

    // 1. Authoritative Verified Result Recording
    let match_id = String::from_str(&env, "match-xyz-001");
    let result_hash = BytesN::from_array(&env, &[7u8; 32]);
    let score: u32 = 4250;
    let verification_version: u32 = 1;

    let input = VerifiedResultInput {
        tournament_id,
        match_id: match_id.clone(),
        player: player1.clone(),
        result_hash: result_hash.clone(),
        score,
        verification_version,
    };

    let verified_record = client.record_verified_result(&verifier, &input);

    assert_eq!(verified_record.score, 4250);
    assert_eq!(verified_record.player, player1);
    assert_eq!(verified_record.result_hash, result_hash);

    let queried = client.get_verified_result(&tournament_id, &match_id);
    assert_eq!(queried.score, 4250);

    // Duplicate match result submission is rejected
    let dup_res = client.try_record_verified_result(&verifier, &input);
    assert!(dup_res.is_err());

    // 2. Final Ranking Hash Finalization
    let ranking_hash = BytesN::from_array(&env, &[9u8; 32]);
    let rank_record = client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);
    assert_eq!(rank_record.ranking_hash, ranking_hash);

    // Duplicate ranking finalization is rejected
    let dup_rank = client.try_record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);
    assert!(dup_rank.is_err());
}

#[test]
fn test_prize_agreement_lifecycle_and_immutability() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);

    let prize_amount: i128 = 100_000_000;
    token_asset.mint(&funder, &prize_amount);

    let tournament_id: u64 = 501;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Agreement Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 16,
        start_time: 1000,
        end_time: 2000,
    };
    client.create_tournament(&creator, &config);

    // 1. Register Prize Agreement Version 1
    let agreement_hash = BytesN::from_array(&env, &[3u8; 32]);
    let agr_v1 = client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    assert_eq!(agr_v1.status, AgreementStatus::PendingApproval);
    assert!(!client.is_agreement_locked(&tournament_id, &1));

    // 2. Lock Prize Agreement Version 1
    let locked_v1 = client.lock_prize_agreement(&creator, &tournament_id, &1);
    assert_eq!(locked_v1.status, AgreementStatus::Locked);
    assert!(client.is_agreement_locked(&tournament_id, &1));

    // 3. Mutating or locking an already locked agreement is rejected (Immutability Invariant)
    let re_lock = client.try_lock_prize_agreement(&creator, &tournament_id, &1);
    assert!(re_lock.is_err());

    // 4. Duplicate version registration rejected
    let dup_ver =
        client.try_register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    assert!(dup_ver.is_err());

    // 5. Unauthorized wallet cannot lock agreement
    let impostor = Address::generate(&env);
    let unauth_lock = client.try_lock_prize_agreement(&impostor, &tournament_id, &1);
    assert!(unauth_lock.is_err());
}

#[test]
fn test_capacity_and_unauthorized_cases() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let prize_amount: i128 = 50_000_000;
    token_asset.mint(&funder, &prize_amount);

    let tournament_id: u64 = 202;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Mini Duel"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 1,
        start_time: 100,
        end_time: 200,
    };
    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    client.open_tournament(&creator, &tournament_id);

    client.register_participant(&p1, &tournament_id);

    // Exceeding capacity fails
    let p2_res = client.try_register_participant(&p2, &tournament_id);
    assert!(p2_res.is_err());
}

#[test]
fn test_zero_amount_and_invalid_bounds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    let config_zero = TournamentConfig {
        tournament_id: 301,
        title: String::from_str(&env, "Zero Prize"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token.clone(),
        target_prize_amount: 0,
        max_participants: 10,
        start_time: 100,
        end_time: 200,
    };
    assert!(client
        .try_create_tournament(&creator, &config_zero)
        .is_err());
}

#[test]
fn test_settlement_authorization_and_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let winner1 = Address::generate(&env);
    let winner2 = Address::generate(&env);
    let winner3 = Address::generate(&env);

    let prize_pool: i128 = 100_000_000; // 100 tokens
    token_asset.mint(&funder, &prize_pool);

    let tournament_id: u64 = 501;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Settlement Grand Prix"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 10,
        start_time: 100,
        end_time: 200,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    client.complete_tournament(&creator, &tournament_id);

    // Commit final ranking
    let ranking_hash = BytesN::from_array(&env, &[0x42; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    // Register and lock prize agreement v1
    let agreement_hash = BytesN::from_array(&env, &[0x88; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    // Formulate deterministic settlement: 50% / 30% / 20%
    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner1.clone(),
        rank: 1,
        amount: 50_000_000,
    });
    allocations.push_back(RecipientAllocation {
        recipient: winner2.clone(),
        rank: 2,
        amount: 30_000_000,
    });
    allocations.push_back(RecipientAllocation {
        recipient: winner3.clone(),
        rank: 3,
        amount: 20_000_000,
    });

    let settlement_hash = BytesN::from_array(&env, &[0x99; 32]);
    let input = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash: agreement_hash.clone(),
        ranking_version: 1,
        ranking_hash: ranking_hash.clone(),
        settlement_hash: settlement_hash.clone(),
        allocations,
    };

    // 1. Authorize settlement
    let record = client.authorize_settlement(&creator, &input);
    assert_eq!(record.status, SettlementStatus::Authorized);
    assert_eq!(record.total_amount, prize_pool);
    assert_eq!(client.is_settled(&tournament_id), false);

    // Initial balances should be 0
    assert_eq!(token_client.balance(&winner1), 0);
    assert_eq!(token_client.balance(&winner2), 0);
    assert_eq!(token_client.balance(&winner3), 0);

    // 2. Execute multi-recipient settlement from vault
    let settled = client.execute_settlement(&creator, &tournament_id);
    assert_eq!(settled.status, SettlementStatus::Settled);
    assert_eq!(client.is_settled(&tournament_id), true);

    // Verify token transfers landed in winner accounts
    assert_eq!(token_client.balance(&winner1), 50_000_000);
    assert_eq!(token_client.balance(&winner2), 30_000_000);
    assert_eq!(token_client.balance(&winner3), 20_000_000);

    // Vault balance should now be 0
    assert_eq!(client.get_vault_balance(&tournament_id), 0);

    // 3. Double execution fails
    let dup_exec = client.try_execute_settlement(&creator, &tournament_id);
    assert!(dup_exec.is_err());
}

#[test]
fn test_settlement_security_and_rejections() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let winner = Address::generate(&env);

    let prize_pool: i128 = 100_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id: u64 = 601;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Security Audit Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 10,
        start_time: 100,
        end_time: 200,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    client.complete_tournament(&creator, &tournament_id);

    let ranking_hash = BytesN::from_array(&env, &[0x42; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let agreement_hash = BytesN::from_array(&env, &[0x88; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: 100_000_000,
    });

    // 1. Mismatched agreement hash rejected
    let bad_agree_hash = BytesN::from_array(&env, &[0x00; 32]);
    let input_bad_agree = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash: bad_agree_hash,
        ranking_version: 1,
        ranking_hash: ranking_hash.clone(),
        settlement_hash: BytesN::from_array(&env, &[0x99; 32]),
        allocations: allocations.clone(),
    };
    assert!(client
        .try_authorize_settlement(&creator, &input_bad_agree)
        .is_err());

    // 2. Mismatched ranking hash rejected
    let bad_rank_hash = BytesN::from_array(&env, &[0x00; 32]);
    let input_bad_rank = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash: agreement_hash.clone(),
        ranking_version: 1,
        ranking_hash: bad_rank_hash,
        settlement_hash: BytesN::from_array(&env, &[0x99; 32]),
        allocations: allocations.clone(),
    };
    assert!(client
        .try_authorize_settlement(&creator, &input_bad_rank)
        .is_err());

    // 3. Allocations exceeding funded prize pool rejected
    let mut over_allocations = soroban_sdk::Vec::new(&env);
    over_allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: 200_000_000, // Exceeds 100_000_000
    });
    let input_over = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash: agreement_hash.clone(),
        ranking_version: 1,
        ranking_hash: ranking_hash.clone(),
        settlement_hash: BytesN::from_array(&env, &[0x99; 32]),
        allocations: over_allocations,
    };
    assert!(client
        .try_authorize_settlement(&creator, &input_over)
        .is_err());
}
