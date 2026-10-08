#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, String,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    TournamentNotFound = 4,
    TournamentAlreadyExists = 5,
    InvalidTournamentState = 6,
    InvalidAmount = 7,
    InvalidTimeBounds = 8,
    TournamentNotFunded = 9,
    TournamentAlreadyFunded = 10,
    TournamentFull = 11,
    ParticipantAlreadyRegistered = 12,
    TournamentNotOpen = 13,
    ArithmeticOverflow = 14,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TournamentState {
    Created = 0,
    Funding = 1,
    Funded = 2,
    Open = 3,
    InProgress = 4,
    Completed = 5,
    Cancelled = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentConfig {
    pub tournament_id: u64,
    pub title: String,
    pub game_id: String,
    pub prize_asset: Address,
    pub target_prize_amount: i128,
    pub max_participants: u32,
    pub start_time: u64,
    pub end_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentInfo {
    pub tournament_id: u64,
    pub creator: Address,
    pub title: String,
    pub game_id: String,
    pub prize_asset: Address,
    pub target_prize_amount: i128,
    pub current_funded_amount: i128,
    pub max_participants: u32,
    pub current_participants: u32,
    pub start_time: u64,
    pub end_time: u64,
    pub state: TournamentState,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Tournament(u64),
    Participant(u64, Address),
    TournamentCount,
}

#[contract]
pub struct TournamentVaultContract;

#[contractimpl]
impl TournamentVaultContract {
    /// Initialize the contract with an administrative address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::TournamentCount, &0u64);
        Ok(())
    }

    /// Retrieve the admin address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    /// Create a new tournament record.
    pub fn create_tournament(
        env: Env,
        creator: Address,
        config: TournamentConfig,
    ) -> Result<TournamentInfo, Error> {
        creator.require_auth();

        if config.target_prize_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if config.end_time <= config.start_time {
            return Err(Error::InvalidTimeBounds);
        }

        let key = DataKey::Tournament(config.tournament_id);
        if env.storage().persistent().has(&key) {
            return Err(Error::TournamentAlreadyExists);
        }

        let tournament = TournamentInfo {
            tournament_id: config.tournament_id,
            creator: creator.clone(),
            title: config.title,
            game_id: config.game_id,
            prize_asset: config.prize_asset.clone(),
            target_prize_amount: config.target_prize_amount,
            current_funded_amount: 0,
            max_participants: config.max_participants,
            current_participants: 0,
            start_time: config.start_time,
            end_time: config.end_time,
            state: TournamentState::Funding,
        };

        env.storage().persistent().set(&key, &tournament);

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TournamentCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TournamentCount, &(count + 1));

        // Emit tournament_created event
        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("created")),
            (
                config.tournament_id,
                creator,
                config.prize_asset,
                config.target_prize_amount,
            ),
        );

        Ok(tournament)
    }

    /// Fund the prize pool for a tournament.
    /// Transfers tokens from the funder directly into this contract's vault.
    pub fn fund_prize_pool(
        env: Env,
        funder: Address,
        tournament_id: u64,
        amount: i128,
    ) -> Result<TournamentInfo, Error> {
        funder.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.state != TournamentState::Created
            && tournament.state != TournamentState::Funding
        {
            return Err(Error::InvalidTournamentState);
        }

        let remaining = tournament
            .target_prize_amount
            .checked_sub(tournament.current_funded_amount)
            .ok_or(Error::ArithmeticOverflow)?;

        if remaining <= 0 {
            return Err(Error::TournamentAlreadyFunded);
        }

        let deposit_amount = if amount > remaining {
            remaining
        } else {
            amount
        };

        // Transfer tokens from funder to this contract
        let client = token::Client::new(&env, &tournament.prize_asset);
        client.transfer(&funder, &env.current_contract_address(), &deposit_amount);

        tournament.current_funded_amount = tournament
            .current_funded_amount
            .checked_add(deposit_amount)
            .ok_or(Error::ArithmeticOverflow)?;

        if tournament.current_funded_amount >= tournament.target_prize_amount {
            tournament.state = TournamentState::Funded;
        }

        env.storage().persistent().set(&key, &tournament);

        // Emit prize_pool_funded event
        env.events().publish(
            (symbol_short!("prize"), symbol_short!("funded")),
            (
                tournament_id,
                funder,
                deposit_amount,
                tournament.current_funded_amount,
            ),
        );

        Ok(tournament)
    }

    /// Open tournament for registration after funding is complete.
    pub fn open_tournament(
        env: Env,
        caller: Address,
        tournament_id: u64,
    ) -> Result<TournamentInfo, Error> {
        caller.require_auth();

        let key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            return Err(Error::Unauthorized);
        }

        if tournament.state != TournamentState::Funded {
            return Err(Error::TournamentNotFunded);
        }

        tournament.state = TournamentState::Open;
        env.storage().persistent().set(&key, &tournament);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("opened")),
            tournament_id,
        );

        Ok(tournament)
    }

    /// Register a participant into an open tournament.
    pub fn register_participant(
        env: Env,
        player: Address,
        tournament_id: u64,
    ) -> Result<(), Error> {
        player.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.state != TournamentState::Open {
            return Err(Error::TournamentNotOpen);
        }

        if tournament.max_participants > 0
            && tournament.current_participants >= tournament.max_participants
        {
            return Err(Error::TournamentFull);
        }

        let part_key = DataKey::Participant(tournament_id, player.clone());
        if env.storage().persistent().has(&part_key) {
            return Err(Error::ParticipantAlreadyRegistered);
        }

        env.storage().persistent().set(&part_key, &true);

        tournament.current_participants = tournament
            .current_participants
            .checked_add(1)
            .ok_or(Error::ArithmeticOverflow)?;

        env.storage().persistent().set(&tourn_key, &tournament);

        env.events().publish(
            (symbol_short!("part"), symbol_short!("joined")),
            (tournament_id, player),
        );

        Ok(())
    }

    /// Transition tournament to InProgress state.
    pub fn start_tournament(
        env: Env,
        caller: Address,
        tournament_id: u64,
    ) -> Result<TournamentInfo, Error> {
        caller.require_auth();

        let key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            return Err(Error::Unauthorized);
        }

        if tournament.state != TournamentState::Open {
            return Err(Error::InvalidTournamentState);
        }

        tournament.state = TournamentState::InProgress;
        env.storage().persistent().set(&key, &tournament);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("started")),
            tournament_id,
        );

        Ok(tournament)
    }

    /// Complete tournament lifecycle.
    pub fn complete_tournament(
        env: Env,
        caller: Address,
        tournament_id: u64,
    ) -> Result<TournamentInfo, Error> {
        caller.require_auth();

        let key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            return Err(Error::Unauthorized);
        }

        if tournament.state != TournamentState::InProgress
            && tournament.state != TournamentState::Open
        {
            return Err(Error::InvalidTournamentState);
        }

        tournament.state = TournamentState::Completed;
        env.storage().persistent().set(&key, &tournament);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("done")),
            tournament_id,
        );

        Ok(tournament)
    }

    /// Fetch tournament details.
    pub fn get_tournament(env: Env, tournament_id: u64) -> Result<TournamentInfo, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Tournament(tournament_id))
            .ok_or(Error::TournamentNotFound)
    }

    /// Check if player is registered in a tournament.
    pub fn is_participant(env: Env, tournament_id: u64, player: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Participant(tournament_id, player))
    }

    /// Get current vault prize balance for a tournament.
    pub fn get_vault_balance(env: Env, tournament_id: u64) -> Result<i128, Error> {
        let t = Self::get_tournament(env, tournament_id)?;
        Ok(t.current_funded_amount)
    }
}

#[cfg(test)]
mod test;
