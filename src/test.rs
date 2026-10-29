#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token::Client as TokenClient,
    token::StellarAssetClient,
    Address, BytesN, Env, String,
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

    client.extend_contract_instance_ttl();

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
    assert_eq!(client.get_vault_funder(&tournament_id), funder);
    client.open_tournament(&creator, &tournament_id);

    // Register players
    client.register_participant(&player1, &tournament_id);
    client.register_participant(&player2, &tournament_id);

    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(1700086401);
    client.complete_tournament(&creator, &tournament_id);

    client.extend_tournament_ttl(&tournament_id);

    assert_eq!(client.get_vault_balance(&tournament_id), prize_amount);
    assert_eq!(client.get_participant_count(&tournament_id), 2);
    assert_eq!(
        client.get_tournament_status(&tournament_id),
        TournamentState::Completed
    );

    let retrieved_cfg = client.get_tournament_config(&tournament_id);
    assert_eq!(retrieved_cfg.tournament_id, tournament_id);
    assert_eq!(retrieved_cfg.target_prize_amount, prize_amount);
    assert_eq!(retrieved_cfg.max_participants, 10);

    let page1 = client.get_participants_page(&tournament_id, &0, &1);
    assert_eq!(page1.len(), 1);
    assert_eq!(page1.get(0).unwrap(), player1);

    let page2 = client.get_participants_page(&tournament_id, &1, &1);
    assert_eq!(page2.len(), 1);
    assert_eq!(page2.get(0).unwrap(), player2);

    let page_empty = client.get_participants_page(&tournament_id, &5, &10);
    assert_eq!(page_empty.len(), 0);
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

    assert_eq!(
        client.get_agreement_status(&tournament_id, &1),
        AgreementStatus::Locked
    );
    assert_eq!(client.get_active_agreement_version(&tournament_id), 1);

    // 6. Test supersede transition on revision
    let tourn_id_2: u64 = 502;
    let config_2 = TournamentConfig {
        tournament_id: tourn_id_2,
        title: String::from_str(&env, "Supersede Test Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 16,
        start_time: 1000,
        end_time: 2000,
    };
    client.create_tournament(&creator, &config_2);
    let hash_v1 = BytesN::from_array(&env, &[1u8; 32]);
    let hash_v2 = BytesN::from_array(&env, &[2u8; 32]);
    client.register_prize_agreement(&creator, &tourn_id_2, &1, &hash_v1);
    assert_eq!(
        client.get_agreement_status(&tourn_id_2, &1),
        AgreementStatus::PendingApproval
    );
    assert_eq!(client.get_active_agreement_version(&tourn_id_2), 1);

    client.register_prize_agreement(&creator, &tourn_id_2, &2, &hash_v2);
    assert_eq!(
        client.get_agreement_status(&tourn_id_2, &1),
        AgreementStatus::Superseded
    );
    assert_eq!(
        client.get_agreement_status(&tourn_id_2, &2),
        AgreementStatus::PendingApproval
    );
    assert_eq!(client.get_active_agreement_version(&tourn_id_2), 2);
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

    // Exceeding capacity fails with TournamentFull
    let p2_res = client.try_register_participant(&p2, &tournament_id);
    assert_eq!(p2_res, Err(Ok(Error::TournamentFull)));
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
    env.ledger().set_timestamp(201);
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
    env.ledger().set_timestamp(201);
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

#[test]
fn test_emergency_pause_circuit_breaker() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.is_paused(), false);

    // Pause contract
    client.pause();
    assert_eq!(client.is_paused(), true);

    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let config = TournamentConfig {
        tournament_id: 999,
        title: String::from_str(&env, "Paused Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token,
        target_prize_amount: 10_000_000,
        max_participants: 5,
        start_time: 1000,
        end_time: 2000,
    };

    // Creating tournament should fail when paused
    assert_eq!(
        client.try_create_tournament(&creator, &config),
        Err(Ok(Error::ContractPaused))
    );

    // Unpause contract
    client.unpause();
    assert_eq!(client.is_paused(), false);

    // Creating tournament should now succeed
    let tourn = client.create_tournament(&creator, &config);
    assert_eq!(tourn.tournament_id, 999);
}

#[test]
fn test_tournament_cancel_and_refund() {
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
    let prize: i128 = 50_000_000;
    token_asset.mint(&funder, &prize);

    let tournament_id: u64 = 777;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Refundable Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize);

    assert_eq!(client.get_vault_balance(&tournament_id), prize);

    // Cancel tournament
    let cancelled = client.cancel_tournament(&creator, &tournament_id);
    assert_eq!(cancelled.state, TournamentState::Cancelled);

    // Creator claims refund
    let refunded = client.claim_refund(&creator, &tournament_id);
    assert_eq!(refunded, prize);
    assert_eq!(client.get_vault_balance(&tournament_id), 0);
    assert_eq!(token_client.balance(&creator), prize);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // Error::InvalidAmount = 7
fn test_min_prize_amount_rejection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &token_admin);
    let creator = Address::generate(&env);

    // Target prize is less than MIN_PRIZE_AMOUNT (10_000_000)
    let config = TournamentConfig {
        tournament_id: 888,
        title: String::from_str(&env, "Dust Spam Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: 5_000_000,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
}

#[test]
fn test_sep41_allowance_funding() {
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
    let relayer = Address::generate(&env);

    let prize: i128 = 20_000_000;
    token_asset.mint(&funder, &prize);

    // Funder approves relayer as spender for allowance
    token_client.approve(&funder, &relayer, &prize, &1000);

    let tournament_id: u64 = 901;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Delegated Funding Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);

    // Relayer executes funding on behalf of funder using allowance
    let funded = client.fund_prize_pool_from(&relayer, &funder, &tournament_id, &prize);
    assert_eq!(funded.state, TournamentState::Funded);
    assert_eq!(funded.current_funded_amount, prize);
    assert_eq!(client.get_vault_balance(&tournament_id), prize);
    assert_eq!(token_client.balance(&contract_id), prize);
    assert_eq!(token_client.balance(&funder), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #31)")]
fn test_premature_completion_rejection() {
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

    let prize: i128 = 20_000_000;
    token_asset.mint(&funder, &prize);

    let tournament_id: u64 = 999;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Premature Completion Guard Test"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize,
        max_participants: 10,
        start_time: 1000,
        end_time: 5000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);

    // Ledger timestamp is 2000, which is before end_time 5000 and max capacity 10 is not reached
    env.ledger().set_timestamp(2000);
    client.complete_tournament(&creator, &tournament_id);
}

#[test]
fn test_contract_metadata_descriptor() {
    let env = Env::default();
    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let meta = client.get_contract_metadata();
    assert_eq!(
        meta.name,
        String::from_str(&env, "Stellar Splash Tournament Vault")
    );
    assert_eq!(meta.version, String::from_str(&env, "1.0.0"));
    assert_eq!(meta.protocol_version, 1);
    assert_eq!(meta.min_prize_amount, 10_000_000);
}

#[test]
fn test_max_string_length_rejection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    // 130 character title (> 128 characters)
    let long_title = String::from_str(
        &env,
        "This is an extraordinarily bloated tournament title designed intentionally to exceed the maximum permissible 128 byte limit 1234567890",
    );

    let config = TournamentConfig {
        tournament_id: 888,
        title: long_title,
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token,
        target_prize_amount: 10_000_000,
        max_participants: 5,
        start_time: 1000,
        end_time: 2000,
    };

    assert_eq!(
        client.try_create_tournament(&creator, &config),
        Err(Ok(Error::TitleTooLong))
    );
}

#[test]
fn test_tournament_details_treasury_and_settled_amount() {
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
    let treasury_addr = Address::generate(&env);

    // Initial total settled amount should be zero
    assert_eq!(client.get_total_settled_amount(), 0);

    // Verify initial treasury fee bps is 0
    assert_eq!(client.get_treasury_fee_bps(), 0);

    // Set treasury
    client.set_treasury(&treasury_addr, &250);
    assert_eq!(client.get_treasury_address(), Some(treasury_addr.clone()));
    assert_eq!(client.get_treasury_fee_bps(), 250);

    // Reject treasury fee bps exceeding MAX_TREASURY_FEE_BPS (1_000)
    assert_eq!(
        client.try_set_treasury(&treasury_addr, &1_001),
        Err(Ok(Error::InvalidTreasuryFeeBps))
    );

    // Duration guard: reject tournament with duration < MIN_TOURNAMENT_DURATION (60s)
    let too_short_config = TournamentConfig {
        tournament_id: 301,
        title: String::from_str(&env, "Too Short Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: 10_000_000,
        max_participants: 5,
        start_time: 1000,
        end_time: 1059, // 59s duration < 60s
    };
    assert_eq!(
        client.try_create_tournament(&creator, &too_short_config),
        Err(Ok(Error::TournamentDurationTooShort))
    );

    // Valid tournament
    let prize_pool: i128 = 50_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id: u64 = 302;
    let valid_config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Details & Settlement Test"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 8,
        start_time: 1000,
        end_time: 1060, // Exactly 60s
    };

    client.create_tournament(&creator, &valid_config);

    // Query tournament details view
    let details = client.get_tournament_details(&tournament_id);
    assert_eq!(details.tournament_id, tournament_id);
    assert_eq!(details.state, TournamentState::Funding);
    assert_eq!(details.target_prize_amount, prize_pool);
    assert_eq!(details.current_funded_amount, 0);
    assert_eq!(details.current_participants, 0);
    assert_eq!(details.max_participants, 8);
    assert_eq!(details.start_time, 1000);
    assert_eq!(details.end_time, 1060);

    // Progress through lifecycle to settlement
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(1061);
    client.complete_tournament(&creator, &tournament_id);

    let ranking_hash = BytesN::from_array(&env, &[0x55; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let agreement_hash = BytesN::from_array(&env, &[0x77; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: prize_pool,
    });

    let settlement_hash = BytesN::from_array(&env, &[0x88; 32]);
    let input = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash,
        ranking_version: 1,
        ranking_hash,
        settlement_hash,
        allocations,
    };

    client.authorize_settlement(&creator, &input);
    client.execute_settlement(&creator, &tournament_id);

    // Verify get_total_settled_amount updated to reflect settled prize
    assert_eq!(client.get_total_settled_amount(), prize_pool);
    assert_eq!(token_client.balance(&winner), prize_pool);
}

#[test]
fn test_metadata_uri_participant_bounds_and_dispute_window() {
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

    // 1. Participant bounds rejection: max_participants = 0
    let zero_part_config = TournamentConfig {
        tournament_id: 401,
        title: String::from_str(&env, "Zero Capacity Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: 10_000_000,
        max_participants: 0,
        start_time: 1000,
        end_time: 2000,
    };
    assert_eq!(
        client.try_create_tournament(&creator, &zero_part_config),
        Err(Ok(Error::InvalidMaxParticipants))
    );

    // 2. Participant bounds rejection: max_participants > 10,000
    let over_part_config = TournamentConfig {
        tournament_id: 402,
        title: String::from_str(&env, "Excess Capacity Tournament"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: 10_000_000,
        max_participants: 10_001,
        start_time: 1000,
        end_time: 2000,
    };
    assert_eq!(
        client.try_create_tournament(&creator, &over_part_config),
        Err(Ok(Error::InvalidMaxParticipants))
    );

    // 3. Asset Whitelisting
    assert_eq!(client.is_asset_whitelisted(&token_client.address), false);
    client.set_asset_whitelisted(&token_client.address, &true);
    assert_eq!(client.is_asset_whitelisted(&token_client.address), true);
    client.set_asset_whitelisted(&token_client.address, &false);
    assert_eq!(client.is_asset_whitelisted(&token_client.address), false);

    // 4. Valid tournament creation & metadata URI
    let prize_pool: i128 = 25_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id: u64 = 403;
    let valid_config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Dispute Window Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &valid_config);

    // Initial metadata URI is None
    assert_eq!(client.get_tournament_metadata_uri(&tournament_id), None);

    let ipfs_uri = String::from_str(
        &env,
        "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    );
    client.set_tournament_metadata_uri(&creator, &tournament_id, &ipfs_uri);
    assert_eq!(
        client.get_tournament_metadata_uri(&tournament_id),
        Some(ipfs_uri)
    );

    // Progress through funding, opening, completion
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(2001);
    client.complete_tournament(&creator, &tournament_id);

    let ranking_hash = BytesN::from_array(&env, &[0x33; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let agreement_hash = BytesN::from_array(&env, &[0x44; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: prize_pool,
    });

    let settlement_hash = BytesN::from_array(&env, &[0x55; 32]);
    let input = SettlementInput {
        tournament_id,
        agreement_version: 1,
        agreement_hash,
        ranking_version: 1,
        ranking_hash,
        settlement_hash,
        allocations,
    };

    client.authorize_settlement(&creator, &input);

    // 5. Dispute Window testing
    // Opening dispute window with duration < MIN_DISPUTE_WINDOW_SECONDS (300s) fails
    assert_eq!(
        client.try_open_dispute_window(&creator, &tournament_id, &299),
        Err(Ok(Error::TournamentDurationTooShort))
    );

    // Open dispute window of 300s at ledger timestamp 2001 -> ends at 2301
    let end_ts = client.open_dispute_window(&creator, &tournament_id, &300);
    assert_eq!(end_ts, 2301);
    assert_eq!(client.get_dispute_window_end(&tournament_id), Some(2301));
    assert_eq!(client.is_dispute_window_elapsed(&tournament_id), false);

    // Attempting settlement before dispute window elapses fails with DisputeWindowNotElapsed
    assert_eq!(
        client.try_execute_settlement(&creator, &tournament_id),
        Err(Ok(Error::DisputeWindowNotElapsed))
    );

    // Fast-forward ledger timestamp to 2301 (dispute window elapsed)
    env.ledger().set_timestamp(2301);
    assert_eq!(client.is_dispute_window_elapsed(&tournament_id), true);

    // Settlement now executes successfully
    let settled = client.execute_settlement(&creator, &tournament_id);
    assert_eq!(settled.status, SettlementStatus::Settled);
    assert_eq!(token_client.balance(&winner), prize_pool);
}
