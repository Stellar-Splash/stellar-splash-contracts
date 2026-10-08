#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _, token::Client as TokenClient, token::StellarAssetClient, Address, Env,
    String,
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

    // Mint tokens to funder
    let prize_amount: i128 = 100_000_000; // 10 XLM/USDC with 7 decimals
    token_asset.mint(&funder, &prize_amount);
    assert_eq!(token_client.balance(&funder), prize_amount);

    // 1. Create Tournament
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

    assert_eq!(tourn.tournament_id, tournament_id);
    assert_eq!(tourn.state, TournamentState::Funding);
    assert_eq!(tourn.current_funded_amount, 0);

    // 2. Attempt to open before funding -> should fail
    let open_res = client.try_open_tournament(&creator, &tournament_id);
    assert!(open_res.is_err());

    // 3. Fund prize pool
    let funded_tourn = client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    assert_eq!(funded_tourn.state, TournamentState::Funded);
    assert_eq!(funded_tourn.current_funded_amount, prize_amount);
    assert_eq!(client.get_vault_balance(&tournament_id), prize_amount);
    assert_eq!(token_client.balance(&contract_id), prize_amount);
    assert_eq!(token_client.balance(&funder), 0);

    // 4. Open tournament
    let opened = client.open_tournament(&creator, &tournament_id);
    assert_eq!(opened.state, TournamentState::Open);

    // 5. Register participants
    client.register_participant(&player1, &tournament_id);
    client.register_participant(&player2, &tournament_id);

    assert!(client.is_participant(&tournament_id, &player1));
    assert!(client.is_participant(&tournament_id, &player2));

    // Duplicate registration should fail
    let dup_res = client.try_register_participant(&player1, &tournament_id);
    assert!(dup_res.is_err());

    // 6. Start tournament
    let in_progress = client.start_tournament(&creator, &tournament_id);
    assert_eq!(in_progress.state, TournamentState::InProgress);

    // 7. Complete tournament
    let completed = client.complete_tournament(&creator, &tournament_id);
    assert_eq!(completed.state, TournamentState::Completed);

    // Vault balance remains safely locked
    assert_eq!(client.get_vault_balance(&tournament_id), prize_amount);
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
        max_participants: 1, // Max participants = 1
        start_time: 100,
        end_time: 200,
    };
    client.create_tournament(&creator, &config);

    client.fund_prize_pool(&funder, &tournament_id, &prize_amount);
    client.open_tournament(&creator, &tournament_id);

    // p1 joins -> success
    client.register_participant(&p1, &tournament_id);

    // p2 joins -> should fail because max_participants = 1
    let p2_res = client.try_register_participant(&p2, &tournament_id);
    assert!(p2_res.is_err());

    // Non-creator opening or completing should fail
    let non_creator = Address::generate(&env);
    let unauth_start = client.try_start_tournament(&non_creator, &tournament_id);
    assert!(unauth_start.is_err());
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

    // Zero prize amount should fail
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

    // Negative prize amount should fail
    let config_neg = TournamentConfig {
        tournament_id: 302,
        title: String::from_str(&env, "Neg Prize"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token.clone(),
        target_prize_amount: -1000,
        max_participants: 10,
        start_time: 100,
        end_time: 200,
    };
    assert!(client.try_create_tournament(&creator, &config_neg).is_err());

    // Invalid time bounds (end <= start) should fail
    let config_time = TournamentConfig {
        tournament_id: 303,
        title: String::from_str(&env, "Bad Time"),
        game_id: String::from_str(&env, "splash-rush-v1"),
        prize_asset: token,
        target_prize_amount: 10_000,
        max_participants: 10,
        start_time: 500,
        end_time: 500,
    };
    assert!(client
        .try_create_tournament(&creator, &config_time)
        .is_err());
}

#[test]
fn test_double_initialization_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TournamentVaultContract, ());
    let client = TournamentVaultContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    assert!(client.try_initialize(&admin1).is_ok());
    assert!(client.try_initialize(&admin2).is_err());
}
