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

#[test]
fn test_total_volume_and_vault_balance() {
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

    assert_eq!(client.get_total_volume(), 0);
    assert_eq!(client.get_contract_token_balance(&token_client.address), 0);

    let prize_pool: i128 = 50_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id = 501;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Volume Tracking Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);

    // Verify TotalVolume and VaultBalance
    assert_eq!(client.get_total_volume(), prize_pool);
    assert_eq!(
        client.get_contract_token_balance(&token_client.address),
        prize_pool
    );
}

#[test]
fn test_dispute_submission_and_resolution() {
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
    let challenger = Address::generate(&env);
    let winner = Address::generate(&env);

    let prize_pool: i128 = 30_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id = 601;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Dispute Resolution Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(2001);
    client.complete_tournament(&creator, &tournament_id);

    let ranking_hash = BytesN::from_array(&env, &[0xaa; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let agreement_hash = BytesN::from_array(&env, &[0xbb; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: prize_pool,
    });

    let settlement_hash = BytesN::from_array(&env, &[0xcc; 32]);
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

    // Open dispute window of 400s (ends at 2401)
    client.open_dispute_window(&creator, &tournament_id, &400);

    // Submit dispute within dispute window
    assert_eq!(client.is_tournament_disputed(&tournament_id), false);
    let dispute = client.submit_dispute(&challenger, &tournament_id, &101);
    assert_eq!(dispute.reason_code, 101);
    assert_eq!(client.is_tournament_disputed(&tournament_id), true);

    // Duplicate dispute from same challenger rejected
    assert_eq!(
        client.try_submit_dispute(&challenger, &tournament_id, &101),
        Err(Ok(Error::DisputeAlreadySubmitted))
    );

    // Fast-forward past dispute window timestamp (2405)
    env.ledger().set_timestamp(2405);
    assert_eq!(client.is_dispute_window_elapsed(&tournament_id), true);

    // Non-admin attempting settlement while tournament is disputed is rejected
    assert_eq!(
        client.try_execute_settlement(&creator, &tournament_id),
        Err(Ok(Error::TournamentDisputed))
    );

    // Admin resolves dispute
    client.resolve_dispute(&admin, &tournament_id);
    assert_eq!(client.is_tournament_disputed(&tournament_id), false);

    // Settlement now executes successfully
    let settled = client.execute_settlement(&creator, &tournament_id);
    assert_eq!(settled.status, SettlementStatus::Settled);
}

#[test]
fn test_upgrade_authorization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.initialize(&admin);

    let dummy_hash = BytesN::from_array(&env, &[0x99; 32]);

    // Non-admin call fails
    assert_eq!(
        client.try_upgrade(&non_admin, &dummy_hash),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_entry_fee_lifecycle() {
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
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let prize_amount: i128 = 20_000_000;
    token_asset.mint(&funder, &prize_amount);

    let tournament_id: u64 = 888;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Entry Fee Arena"),
        game_id: String::from_str(&env, "splash-rush"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);

    // Creator configures entry fee
    let entry_fee_amount: i128 = 5_000_000;
    client.set_entry_fee(
        &creator,
        &tournament_id,
        &token_client.address,
        &entry_fee_amount,
    );

    let fee_config = client.get_entry_fee(&tournament_id);
    assert_eq!(fee_config.fee_amount, entry_fee_amount);
    assert_eq!(fee_config.fee_asset, token_client.address);

    client.open_tournament(&creator, &tournament_id);

    // Mint tokens to players for entry fee
    token_asset.mint(&player1, &entry_fee_amount);
    token_asset.mint(&player2, &entry_fee_amount);

    // Player 1 pays entry fee
    client.pay_entry_fee(&player1, &tournament_id);
    assert_eq!(client.has_paid_entry_fee(&tournament_id, &player1), true);
    assert_eq!(client.has_paid_entry_fee(&tournament_id, &player2), false);
    assert_eq!(
        client.get_total_entry_fees(&tournament_id),
        entry_fee_amount
    );

    // Duplicate payment fails
    assert_eq!(
        client.try_pay_entry_fee(&player1, &tournament_id),
        Err(Ok(Error::EntryFeeAlreadyPaid))
    );

    // Player 2 pays entry fee
    client.pay_entry_fee(&player2, &tournament_id);
    assert_eq!(client.has_paid_entry_fee(&tournament_id, &player2), true);
    assert_eq!(
        client.get_total_entry_fees(&tournament_id),
        entry_fee_amount * 2
    );

    // Contract vault token balance is prize_amount + 2 * entry_fee_amount
    assert_eq!(
        client.get_contract_token_balance(&token_client.address),
        prize_amount + entry_fee_amount * 2
    );
}

#[test]
fn test_player_stats_and_active_tournaments() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let player = Address::generate(&env);

    // Initial stats should be zeroed
    let init_stats = client.get_player_stats(&player);
    assert_eq!(init_stats.tournaments_joined, 0);
    assert_eq!(init_stats.tournaments_won, 0);

    let prize_amount: i128 = 20_000_000;
    token_asset.mint(&funder, &(prize_amount * 2));

    let t1_id: u64 = 501;
    let config1 = TournamentConfig {
        tournament_id: t1_id,
        title: String::from_str(&env, "Tournament Alpha"),
        game_id: String::from_str(&env, "splash-rush"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };
    client.create_tournament(&creator, &config1);
    client.fund_prize_pool(&funder, &t1_id, &prize_amount);
    client.open_tournament(&creator, &t1_id);

    let t2_id: u64 = 502;
    let config2 = TournamentConfig {
        tournament_id: t2_id,
        title: String::from_str(&env, "Tournament Beta"),
        game_id: String::from_str(&env, "splash-rush"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };
    client.create_tournament(&creator, &config2);

    let active_tournaments = client.get_active_tournaments();
    assert_eq!(active_tournaments.contains(t1_id), true);
    assert_eq!(active_tournaments.contains(t2_id), true);

    // Register participant and verify tournaments_joined increment
    client.register_participant(&player, &t1_id);
    let joined_stats = client.get_player_stats(&player);
    assert_eq!(joined_stats.tournaments_joined, 1);
    assert_eq!(joined_stats.tournaments_won, 0);

    // Non-admin cannot increment wins
    assert_eq!(
        client.try_increment_player_wins(&non_admin, &player),
        Err(Ok(Error::Unauthorized))
    );

    // Admin records tournament victory
    client.increment_player_wins(&admin, &player);
    let won_stats = client.get_player_stats(&player);
    assert_eq!(won_stats.tournaments_won, 1);
}

#[test]
fn test_dispute_dismissal_and_contract_version() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    // 1. Contract version verification
    assert_eq!(client.get_contract_version(), 21);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let challenger = Address::generate(&env);

    let prize_pool: i128 = 20_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id = 701;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Dispute Dismissal Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 8,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(2001);
    client.complete_tournament(&creator, &tournament_id);

    // Initial dispute status before filing
    assert_eq!(client.get_dispute_status(&tournament_id), None);

    // Open dispute window & submit dispute
    client.open_dispute_window(&creator, &tournament_id, &360);
    let dispute = client.submit_dispute(&challenger, &tournament_id, &102);
    assert_eq!(dispute.reason_code, 102);
    assert_eq!(dispute.status, DisputeStatus::Active);
    assert_eq!(client.is_tournament_disputed(&tournament_id), true);

    // Status query returns pending record
    let status = client
        .get_dispute_status(&tournament_id)
        .expect("dispute record exists");
    assert_eq!(status.status, DisputeStatus::Active);
    assert_eq!(status.reason_code, 102);
    assert_eq!(status.challenger, challenger);

    // Non-admin attempting to dismiss dispute fails
    assert_eq!(
        client.try_dismiss_dispute(&non_admin, &tournament_id),
        Err(Ok(Error::Unauthorized))
    );

    // Admin dismisses dispute
    client.dismiss_dispute(&admin, &tournament_id);
    assert_eq!(client.is_tournament_disputed(&tournament_id), false);

    // Status query shows Dismissed
    let dismissed_status = client
        .get_dispute_status(&tournament_id)
        .expect("dispute record exists");
    assert_eq!(dismissed_status.status, DisputeStatus::Dismissed);
}

#[test]
fn test_dispute_window_extension_and_settlement_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let winner = Address::generate(&env);

    let prize_pool: i128 = 25_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id = 801;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Extension & Queries Cup"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 8,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(2001);
    client.complete_tournament(&creator, &tournament_id);

    // Initial query states before window or settlement
    assert_eq!(client.get_tournament_dispute_window(&tournament_id), None);
    assert_eq!(client.get_settled_amount(&tournament_id), 0);
    assert_eq!(
        client.get_tournament_settlement_status(&tournament_id),
        None
    );

    // Open dispute window of 300s (ledger ts 2001 -> ends at 2301)
    client.open_dispute_window(&creator, &tournament_id, &300);
    let window_info = client
        .get_tournament_dispute_window(&tournament_id)
        .expect("window info exists");
    assert_eq!(window_info.end_timestamp, 2301);
    assert_eq!(window_info.is_elapsed, false);

    // Non-admin cannot extend dispute window
    assert_eq!(
        client.try_extend_dispute_window(&non_admin, &tournament_id, &150),
        Err(Ok(Error::Unauthorized))
    );

    // Admin extends dispute window by 150s (2301 + 150 = 2451)
    let new_end = client.extend_dispute_window(&admin, &tournament_id, &150);
    assert_eq!(new_end, 2451);

    let extended_info = client
        .get_tournament_dispute_window(&tournament_id)
        .expect("extended window info exists");
    assert_eq!(extended_info.end_timestamp, 2451);
    assert_eq!(extended_info.is_elapsed, false);

    // Authorize and execute settlement after dispute window elapses
    let ranking_hash = BytesN::from_array(&env, &[0x11; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let agreement_hash = BytesN::from_array(&env, &[0x22; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let mut allocations = soroban_sdk::Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: winner.clone(),
        rank: 1,
        amount: prize_pool,
    });

    let settlement_hash = BytesN::from_array(&env, &[0x33; 32]);
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

    // Fast-forward past extended dispute window (2455)
    env.ledger().set_timestamp(2455);
    let post_window = client
        .get_tournament_dispute_window(&tournament_id)
        .expect("post window exists");
    assert_eq!(post_window.is_elapsed, true);

    // Execute settlement
    client.execute_settlement(&creator, &tournament_id);

    // Verify settlement queries
    assert_eq!(client.get_settled_amount(&tournament_id), prize_pool);
    assert_eq!(
        client.get_tournament_settlement_status(&tournament_id),
        Some(SettlementStatus::Settled)
    );
}

#[test]
fn test_dispute_active_capacity_and_treasury_queries() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(2000);

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Verify contract deployer query matches admin
    assert_eq!(client.get_contract_deployer(), admin);

    let token_admin = Address::generate(&env);
    let (_token_client, token_asset) = create_token_contract(&env, &token_admin);

    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let prize_pool: i128 = 20_000_000;
    token_asset.mint(&funder, &prize_pool);

    let tournament_id: u64 = 8801;
    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Capacity Test Cup"),
        game_id: String::from_str(&env, "splash-b12"),
        prize_asset: token_asset.address.clone(),
        target_prize_amount: prize_pool,
        max_participants: 2,
        start_time: 2001,
        end_time: 3000,
    };

    client.create_tournament(&creator, &config);

    // Verify initial capacity info
    let initial_cap = client.get_tournament_capacity_status(&tournament_id);
    assert_eq!(initial_cap.current_participants, 0);
    assert_eq!(initial_cap.max_participants, 2);
    assert_eq!(initial_cap.is_full, false);

    // Treasury fee query before treasury bps configured
    assert_eq!(client.get_tournament_treasury_fee(&tournament_id), 0);

    // Fund tournament
    client.fund_prize_pool(&funder, &tournament_id, &prize_pool);
    client.open_tournament(&creator, &tournament_id);

    // Configure protocol treasury with 500 bps (5.00%)
    let treasury_addr = Address::generate(&env);
    client.set_treasury(&treasury_addr, &500);

    // Treasury fee query should now be 5% of 20_000_000 = 1_000_000
    assert_eq!(
        client.get_tournament_treasury_fee(&tournament_id),
        1_000_000
    );

    // Register 1 player -> not full
    client.register_participant(&player1, &tournament_id);
    let cap_p1 = client.get_tournament_capacity_status(&tournament_id);
    assert_eq!(cap_p1.current_participants, 1);
    assert_eq!(cap_p1.is_full, false);

    // Register 2nd player -> reaches max capacity (2/2)
    client.register_participant(&player2, &tournament_id);
    let cap_p2 = client.get_tournament_capacity_status(&tournament_id);
    assert_eq!(cap_p2.current_participants, 2);
    assert_eq!(cap_p2.is_full, true);

    // Complete tournament & verify dispute activity query
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(3001);
    client.complete_tournament(&creator, &tournament_id);

    // Open dispute window for 300 seconds
    client.open_dispute_window(&creator, &tournament_id, &300);
    assert_eq!(client.is_dispute_active(&tournament_id), false);

    // Submit dispute -> dispute becomes active
    client.submit_dispute(&player1, &tournament_id, &1);
    assert_eq!(client.is_dispute_active(&tournament_id), true);

    // Resolve dispute by admin -> dispute is no longer active
    client.resolve_dispute(&admin, &tournament_id);
    assert_eq!(client.is_dispute_active(&tournament_id), false);
}

#[test]
fn test_batch1_queries_ttl_and_math() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Initial contract stats
    let initial_stats = client.get_contract_stats();
    assert_eq!(initial_stats.total_tournaments, 0);
    assert_eq!(initial_stats.total_settled_amount, 0);
    assert_eq!(initial_stats.total_volume, 0);
    assert_eq!(initial_stats.is_paused, false);

    // Pure math helpers
    assert_eq!(client.calculate_payout_share(&10_000_000, &5000), 5_000_000);
    assert_eq!(client.calculate_payout_share(&10_000_000, &3000), 3_000_000);
    assert_eq!(client.calculate_payout_share(&10_000_000, &0), 0);

    let mut valid_bps = Vec::new(&env);
    valid_bps.push_back(5000);
    valid_bps.push_back(3000);
    valid_bps.push_back(2000);
    assert_eq!(client.validate_basis_points_sum(&valid_bps), true);

    let mut invalid_bps = Vec::new(&env);
    invalid_bps.push_back(5000);
    invalid_bps.push_back(4000);
    assert_eq!(client.validate_basis_points_sum(&invalid_bps), false);

    // Create tournament
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let player1 = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let tournament_id: u64 = 201;
    let prize_amount: i128 = 20_000_000;
    token_asset.mint(&funder, &prize_amount);

    env.ledger().set_timestamp(1000);

    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Batch 1 Championship"),
        game_id: String::from_str(&env, "splash-rush-b1"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 5,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);
    assert_eq!(client.get_tournament_creator(&tournament_id), creator);
    assert_eq!(client.is_tournament_expired(&tournament_id), false);
    assert_eq!(client.get_tournament_time_remaining(&tournament_id), 1000);

    // Fund tournament
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    assert_eq!(
        client.get_tournament_funder_opt(&tournament_id),
        Some(funder.clone())
    );

    // Open tournament
    client.open_tournament(&creator, &tournament_id);

    // Participant checks before & after registration
    assert_eq!(
        client.is_player_registered(&tournament_id, &player1),
        false
    );
    client.register_participant(&player1, &tournament_id);
    assert_eq!(client.is_player_registered(&tournament_id, &player1), true);

    // Storage TTL extension for participant
    assert_eq!(
        client.extend_participant_storage_ttl(&tournament_id, &player1),
        ()
    );

    // Participant pagination calculation
    assert_eq!(
        client.get_participants_page_count(&tournament_id, &2),
        1
    );

    // Tournament summaries query
    let summaries = client.get_tournament_summaries(&0, &10);
    assert_eq!(summaries.len(), 1);
    let s0 = summaries.get(0).unwrap();
    assert_eq!(s0.tournament_id, tournament_id);
    assert_eq!(s0.state, TournamentState::Open);
    assert_eq!(s0.current_participants, 1);

    // Disputed tournaments query (empty initially)
    assert_eq!(client.get_disputed_tournaments().len(), 0);

    // Fast-forward past end time
    env.ledger().set_timestamp(2001);
    assert_eq!(client.is_tournament_expired(&tournament_id), true);
    assert_eq!(client.get_tournament_time_remaining(&tournament_id), 0);

    // Complete tournament
    client.start_tournament(&creator, &tournament_id);
    client.complete_tournament(&creator, &tournament_id);

    // Register agreement & test agreement timestamp & TTL
    let agreement_hash = BytesN::from_array(&env, &[7u8; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);
    assert!(client.get_prize_agreement_timestamp(&tournament_id, &1).is_some());
    assert_eq!(client.extend_agreement_storage_ttl(&tournament_id, &1), ());

    // Record ranking & test ranking timestamp
    let ranking_hash = BytesN::from_array(&env, &[9u8; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);
    assert!(client.get_ranking_timestamp(&tournament_id).is_some());

    // Authorize & Execute settlement
    let mut allocations = Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: player1.clone(),
        rank: 1,
        amount: prize_amount,
    });
    let settlement_hash = BytesN::from_array(&env, &[42u8; 32]);
    client.authorize_settlement(
        &creator,
        &SettlementInput {
            tournament_id,
            agreement_version: 1,
            agreement_hash,
            ranking_version: 1,
            ranking_hash,
            settlement_hash: settlement_hash.clone(),
            allocations,
        },
    );
    client.execute_settlement(&creator, &tournament_id);

    // Verify settlement timestamp & settlement hash match
    assert!(client.get_settlement_timestamp(&tournament_id).is_some());
    assert_eq!(
        client.check_settlement_hash_match(&tournament_id, &settlement_hash),
        true
    );
    let false_hash = BytesN::from_array(&env, &[99u8; 32]);
    assert_eq!(
        client.check_settlement_hash_match(&tournament_id, &false_hash),
        false
    );

    // Storage TTL extension for settlement
    assert_eq!(client.extend_settlement_storage_ttl(&tournament_id), ());

    // Player win rate query
    client.increment_player_wins(&admin, &player1);
    let win_rate = client.get_player_win_rate_bps(&player1);
    assert_eq!(win_rate, 10_000); // 1 win / 1 joined = 10,000 bps (100.00%)
}

#[test]
fn test_batch2_sponsorship_admin_and_payout_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // 1. Admin configurations & queries
    assert_eq!(client.get_min_prize_amount(), MIN_PRIZE_AMOUNT);
    client.set_min_prize_amount(&admin, &15_000_000);
    assert_eq!(client.get_min_prize_amount(), 15_000_000);

    assert_eq!(client.get_max_participants_limit(), MAX_PARTICIPANTS_LIMIT);
    client.set_max_participants_limit(&admin, &500);
    assert_eq!(client.get_max_participants_limit(), 500);

    assert_eq!(
        client.get_default_dispute_window(),
        MIN_DISPUTE_WINDOW_SECONDS
    );
    client.set_dispute_window_duration(&admin, &600);
    assert_eq!(client.get_default_dispute_window(), 600);

    // 2. Setup tokens & actors
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let sponsor1 = Address::generate(&env);
    let sponsor2 = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let tournament_id: u64 = 301;
    let target_prize: i128 = 20_000_000;
    token_asset.mint(&funder, &15_000_000);
    token_asset.mint(&sponsor1, &10_000_000);
    token_asset.mint(&sponsor2, &5_000_000);

    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Batch 2 Sponsored Clash"),
        game_id: String::from_str(&env, "splash-rush-b2"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: target_prize,
        max_participants: 10,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);

    // Title and Game ID queries
    assert_eq!(
        client.get_tournament_title(&tournament_id),
        String::from_str(&env, "Batch 2 Sponsored Clash")
    );
    assert_eq!(
        client.get_tournament_game_id(&tournament_id),
        String::from_str(&env, "splash-rush-b2")
    );

    // Funding deficit check before funding
    assert_eq!(client.is_tournament_funded(&tournament_id), false);
    assert_eq!(client.get_funding_deficit(&tournament_id), target_prize);

    // Fund partial amount (15M of 20M)
    client.fund_prize_pool(&funder, &tournament_id, &15_000_000);
    assert_eq!(client.is_tournament_funded(&tournament_id), false);
    assert_eq!(client.get_funding_deficit(&tournament_id), 5_000_000);

    // Community sponsorship from sponsor1 (5M) -> reaches target!
    client.sponsor_tournament(&sponsor1, &tournament_id, &5_000_000);
    assert_eq!(client.is_tournament_funded(&tournament_id), true);
    assert_eq!(client.get_funding_deficit(&tournament_id), 0);

    // Additional sponsorship from sponsor2 (2M) -> bonus pool!
    client.sponsor_tournament(&sponsor2, &tournament_id, &2_000_000);

    // Check sponsorship queries
    assert_eq!(client.get_tournament_sponsors_count(&tournament_id), 2);
    assert_eq!(
        client.get_sponsor_contribution(&tournament_id, &sponsor1),
        5_000_000
    );
    assert_eq!(
        client.get_sponsor_contribution(&tournament_id, &sponsor2),
        2_000_000
    );
    assert_eq!(
        client.get_total_sponsored_amount(&tournament_id),
        7_000_000
    );

    // Open & register players
    client.open_tournament(&creator, &tournament_id);
    client.register_participant(&player1, &tournament_id);
    client.register_participant(&player2, &tournament_id);

    // Check participant at index
    assert_eq!(
        client.get_participant_at_index(&tournament_id, &0),
        Some(player1.clone())
    );
    assert_eq!(
        client.get_participant_at_index(&tournament_id, &1),
        Some(player2.clone())
    );
    assert_eq!(
        client.get_participant_at_index(&tournament_id, &99),
        None
    );

    // Complete tournament and settle
    client.start_tournament(&creator, &tournament_id);
    env.ledger().set_timestamp(2001);
    client.complete_tournament(&creator, &tournament_id);

    let agreement_hash = BytesN::from_array(&env, &[11u8; 32]);
    client.register_prize_agreement(&creator, &tournament_id, &1, &agreement_hash);
    client.lock_prize_agreement(&creator, &tournament_id, &1);

    let ranking_hash = BytesN::from_array(&env, &[22u8; 32]);
    client.record_ranking_hash(&creator, &tournament_id, &1, &ranking_hash);

    let _total_pool = 15_000_000 + 7_000_000;
    let mut allocations = Vec::new(&env);
    allocations.push_back(RecipientAllocation {
        recipient: player1.clone(),
        rank: 1,
        amount: 15_000_000,
    });
    allocations.push_back(RecipientAllocation {
        recipient: player2.clone(),
        rank: 2,
        amount: 7_000_000,
    });
    let settlement_hash = BytesN::from_array(&env, &[33u8; 32]);
    client.authorize_settlement(
        &creator,
        &SettlementInput {
            tournament_id,
            agreement_version: 1,
            agreement_hash,
            ranking_version: 1,
            ranking_hash,
            settlement_hash,
            allocations,
        },
    );
    client.execute_settlement(&creator, &tournament_id);

    // Verify player settlement queries
    assert_eq!(
        client.check_player_is_winner(&tournament_id, &player1),
        true
    );
    assert_eq!(
        client.check_player_is_winner(&tournament_id, &player2),
        true
    );
    assert_eq!(
        client.check_player_is_winner(&tournament_id, &funder),
        false
    );

    assert_eq!(
        client.get_player_payout_amount(&tournament_id, &player1),
        15_000_000
    );
    assert_eq!(
        client.get_player_payout_amount(&tournament_id, &player2),
        7_000_000
    );
    assert_eq!(
        client.get_player_payout_amount(&tournament_id, &funder),
        0
    );

    assert_eq!(client.get_player_rank(&tournament_id, &player1), 1);
    assert_eq!(client.get_player_rank(&tournament_id, &player2), 2);
    assert_eq!(client.get_player_rank(&tournament_id, &funder), 0);

    // Test sponsor refund on cancelled tournament
    let cancelled_tourn_id: u64 = 302;
    let cancel_config = TournamentConfig {
        tournament_id: cancelled_tourn_id,
        title: String::from_str(&env, "Cancelled Clash"),
        game_id: String::from_str(&env, "splash-rush-b2"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: target_prize,
        max_participants: 10,
        start_time: 3000,
        end_time: 4000,
    };
    client.create_tournament(&creator, &cancel_config);
    token_asset.mint(&sponsor1, &3_000_000);
    client.sponsor_tournament(&sponsor1, &cancelled_tourn_id, &3_000_000);
    client.cancel_tournament(&creator, &cancelled_tourn_id);

    let refunded = client.claim_sponsor_refund(&sponsor1, &cancelled_tourn_id);
    assert_eq!(refunded, 3_000_000);
}

#[test]
fn test_batch3_badges_governance_and_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // 1. Two-step admin transfer
    let proposed_admin = Address::generate(&env);
    client.propose_admin_transfer(&admin, &proposed_admin);
    assert_eq!(client.get_proposed_admin(), Some(proposed_admin.clone()));

    // Cancel transfer
    client.cancel_admin_transfer(&admin);
    assert_eq!(client.get_proposed_admin(), None);

    // Re-propose and accept
    client.propose_admin_transfer(&admin, &proposed_admin);
    client.accept_admin_transfer(&proposed_admin);
    assert_eq!(client.get_admin(), proposed_admin);
    assert_eq!(client.get_proposed_admin(), None);

    // 2. Soulbound Achievement Badges (SBT)
    let player1 = Address::generate(&env);
    let badge = client.award_champion_badge(
        &proposed_admin,
        &player1,
        &401,
        &BadgeType::TournamentChampion,
    );
    assert_eq!(badge.badge_id, 1);
    assert_eq!(badge.player, player1);
    assert_eq!(badge.tournament_id, 401);
    assert_eq!(badge.badge_type, BadgeType::TournamentChampion);

    let badges = client.get_player_badges(&player1);
    assert_eq!(badges.len(), 1);
    assert_eq!(client.get_player_badge_count(&player1), 1);
    assert_eq!(
        client.has_player_badge(&player1, &401, &BadgeType::TournamentChampion),
        true
    );
    assert_eq!(
        client.has_player_badge(&player1, &401, &BadgeType::RunnerUp),
        false
    );

    // 3. Tournament setup & Banner URI & Creator Royalty
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_client, token_asset) = create_token_contract(&env, &token_admin);

    let tournament_id: u64 = 401;
    let prize_amount: i128 = 25_000_000;
    token_asset.mint(&funder, &prize_amount);

    let config = TournamentConfig {
        tournament_id,
        title: String::from_str(&env, "Batch 3 Arena Masters"),
        game_id: String::from_str(&env, "splash-rush-b3"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 8,
        start_time: 1000,
        end_time: 2000,
    };

    client.create_tournament(&creator, &config);

    // Banner URI
    let banner = String::from_str(&env, "ipfs://QmStellarSplashBannerHash");
    client.set_tournament_banner_uri(&creator, &tournament_id, &banner);
    assert_eq!(
        client.get_tournament_banner_uri(&tournament_id),
        Some(banner)
    );

    // Creator Royalty
    client.set_creator_royalty_bps(&creator, &tournament_id, &300); // 3.00%
    assert_eq!(client.get_creator_royalty_bps(&tournament_id), 300);

    // Fund and open
    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    client.open_tournament(&creator, &tournament_id);
    client.register_participant(&player1, &tournament_id);

    // Check royalty calculation on funded vault (3% of 25M = 750k)
    let royalty = client.calculate_creator_royalty(&tournament_id);
    assert_eq!(royalty, 750_000);

    // Asset whitelist query
    client.set_asset_whitelisted(&token_client.address, &true);
    assert_eq!(
        client.check_asset_whitelisted_or_xlm(&token_client.address),
        true
    );

    // Lifecycle status helpers
    assert_eq!(client.is_tournament_in_progress(&tournament_id), false);
    client.start_tournament(&creator, &tournament_id);
    assert_eq!(client.is_tournament_in_progress(&tournament_id), true);

    env.ledger().set_timestamp(2001);
    assert_eq!(client.is_tournament_completed(&tournament_id), false);
    client.complete_tournament(&creator, &tournament_id);
    assert_eq!(client.is_tournament_completed(&tournament_id), true);
    assert_eq!(client.is_tournament_cancelled(&tournament_id), false);

    // Dispute helpers
    client.open_dispute_window(&creator, &tournament_id, &300);
    assert_eq!(client.get_dispute_reason_code(&tournament_id), 0);
    assert_eq!(client.get_dispute_challenger(&tournament_id), None);

    client.submit_dispute(&player1, &tournament_id, &42);
    assert_eq!(client.get_dispute_reason_code(&tournament_id), 42);
    assert_eq!(
        client.get_dispute_challenger(&tournament_id),
        Some(player1)
    );

    // Cancelled tournament check
    let cancel_id: u64 = 402;
    let cancel_config = TournamentConfig {
        tournament_id: cancel_id,
        title: String::from_str(&env, "Cancelled State Test"),
        game_id: String::from_str(&env, "splash-rush-b3"),
        prize_asset: token_client.address.clone(),
        target_prize_amount: prize_amount,
        max_participants: 8,
        start_time: 5000,
        end_time: 6000,
    };
    client.create_tournament(&creator, &cancel_config);
    client.cancel_tournament(&creator, &cancel_id);
    assert_eq!(client.is_tournament_cancelled(&cancel_id), true);
}

#[test]
fn test_formal_invariants_solvency_and_math() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Invariant 1: Valid basis points distribution must sum to exactly 10,000
    let mut valid_bps = Vec::new(&env);
    valid_bps.push_back(5000); // 50%
    valid_bps.push_back(3000); // 30%
    valid_bps.push_back(2000); // 20%
    assert_eq!(client.validate_basis_points_sum(&valid_bps), true);

    // Invariant 2: Invalid basis points (sum != 10,000) must be rejected
    let mut invalid_bps = Vec::new(&env);
    invalid_bps.push_back(5000);
    invalid_bps.push_back(3000);
    invalid_bps.push_back(1999); // 9,999 != 10,000
    assert_eq!(client.validate_basis_points_sum(&invalid_bps), false);

    // Invariant 3: Payout shares must conserve the entire prize pool without leakage
    let prize_pool: i128 = 10_000_000; // 1 XLM in stroops
    let share1 = client.calculate_payout_share(&prize_pool, &5000);
    let share2 = client.calculate_payout_share(&prize_pool, &3000);
    let share3 = client.calculate_payout_share(&prize_pool, &2000);
    assert_eq!(share1 + share2 + share3, prize_pool);

    // Invariant 4: Unconfigured creator royalty defaults strictly to 0
    assert_eq!(client.get_creator_royalty_bps(&101), 0);
    let royalty = client.calculate_creator_royalty(&101);
    assert_eq!(royalty, 0); // 0 vault balance = 0 royalty

    // Invariant 5: Win rate bps bounded between 0 and 10,000
    assert_eq!(client.get_player_win_rate_bps(&admin), 0);
}
