#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env,
    String,
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
    ResultAlreadyRecorded = 15,
    ResultNotFound = 16,
    AgreementAlreadyExists = 17,
    AgreementNotFound = 18,
    AgreementAlreadyLocked = 19,
    AgreementNotLocked = 20,
    RankingAlreadyFinalized = 21,
    RankingNotFound = 22,
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
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AgreementStatus {
    Draft = 0,
    PendingApproval = 1,
    ReadyToLock = 2,
    Locked = 3,
    Superseded = 4,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedResultInput {
    pub tournament_id: u64,
    pub match_id: String,
    pub player: Address,
    pub result_hash: BytesN<32>,
    pub score: u32,
    pub verification_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedResultRecord {
    pub tournament_id: u64,
    pub match_id: String,
    pub player: Address,
    pub result_hash: BytesN<32>,
    pub score: u32,
    pub verification_version: u32,
    pub verified_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankingRecord {
    pub tournament_id: u64,
    pub ranking_version: u32,
    pub ranking_hash: BytesN<32>,
    pub finalized_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrizeAgreementRecord {
    pub tournament_id: u64,
    pub version: u32,
    pub agreement_hash: BytesN<32>,
    pub status: AgreementStatus,
    pub created_at: u64,
    pub locked_at: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Tournament(u64),
    Participant(u64, Address),
    TournamentCount,
    VerifiedResult(u64, String),
    Ranking(u64),
    PrizeAgreement(u64, u32),
    ActiveAgreementVersion(u64),
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

    // -------------------------------------------------------------
    // RESULT ATTESTATION & VERIFICATION LAYER
    // -------------------------------------------------------------

    /// Authoritatively records an independently verified match result on-chain.
    pub fn record_verified_result(
        env: Env,
        verifier: Address,
        input: VerifiedResultInput,
    ) -> Result<VerifiedResultRecord, Error> {
        verifier.require_auth();

        let tourn_key = DataKey::Tournament(input.tournament_id);
        if !env.storage().persistent().has(&tourn_key) {
            return Err(Error::TournamentNotFound);
        }

        let res_key = DataKey::VerifiedResult(input.tournament_id, input.match_id.clone());
        if env.storage().persistent().has(&res_key) {
            return Err(Error::ResultAlreadyRecorded);
        }

        let record = VerifiedResultRecord {
            tournament_id: input.tournament_id,
            match_id: input.match_id.clone(),
            player: input.player.clone(),
            result_hash: input.result_hash.clone(),
            score: input.score,
            verification_version: input.verification_version,
            verified_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&res_key, &record);

        // Emit result_verified event
        env.events().publish(
            (symbol_short!("result"), symbol_short!("verified")),
            (
                input.tournament_id,
                input.match_id,
                input.player,
                input.score,
            ),
        );

        Ok(record)
    }

    /// Retrieve a verified result record.
    pub fn get_verified_result(
        env: Env,
        tournament_id: u64,
        match_id: String,
    ) -> Result<VerifiedResultRecord, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::VerifiedResult(tournament_id, match_id))
            .ok_or(Error::ResultNotFound)
    }

    /// Finalizes the canonical ranking hash for a completed tournament.
    pub fn record_ranking_hash(
        env: Env,
        caller: Address,
        tournament_id: u64,
        ranking_version: u32,
        ranking_hash: BytesN<32>,
    ) -> Result<RankingRecord, Error> {
        caller.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            return Err(Error::Unauthorized);
        }

        let rank_key = DataKey::Ranking(tournament_id);
        if env.storage().persistent().has(&rank_key) {
            return Err(Error::RankingAlreadyFinalized);
        }

        let record = RankingRecord {
            tournament_id,
            ranking_version,
            ranking_hash: ranking_hash.clone(),
            finalized_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&rank_key, &record);

        env.events().publish(
            (symbol_short!("ranking"), symbol_short!("done")),
            (tournament_id, ranking_version, ranking_hash),
        );

        Ok(record)
    }

    /// Retrieve the finalized ranking record.
    pub fn get_ranking_record(env: Env, tournament_id: u64) -> Result<RankingRecord, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Ranking(tournament_id))
            .ok_or(Error::RankingNotFound)
    }

    // -------------------------------------------------------------
    // TOURNAMENT PRIZE AGREEMENT LAYER
    // -------------------------------------------------------------

    /// Register a deterministic prize agreement version.
    pub fn register_prize_agreement(
        env: Env,
        creator: Address,
        tournament_id: u64,
        version: u32,
        agreement_hash: BytesN<32>,
    ) -> Result<PrizeAgreementRecord, Error> {
        creator.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != creator {
            return Err(Error::Unauthorized);
        }

        let agr_key = DataKey::PrizeAgreement(tournament_id, version);
        if env.storage().persistent().has(&agr_key) {
            return Err(Error::AgreementAlreadyExists);
        }

        let record = PrizeAgreementRecord {
            tournament_id,
            version,
            agreement_hash: agreement_hash.clone(),
            status: AgreementStatus::PendingApproval,
            created_at: env.ledger().timestamp(),
            locked_at: 0,
        };

        env.storage().persistent().set(&agr_key, &record);
        env.storage()
            .persistent()
            .set(&DataKey::ActiveAgreementVersion(tournament_id), &version);

        env.events().publish(
            (symbol_short!("agree"), symbol_short!("created")),
            (tournament_id, version, agreement_hash),
        );

        Ok(record)
    }

    /// Authoritatively locks the prize agreement.
    /// Material financial rules are immutable once in Locked state.
    pub fn lock_prize_agreement(
        env: Env,
        creator: Address,
        tournament_id: u64,
        version: u32,
    ) -> Result<PrizeAgreementRecord, Error> {
        creator.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != creator {
            return Err(Error::Unauthorized);
        }

        let agr_key = DataKey::PrizeAgreement(tournament_id, version);
        let mut record: PrizeAgreementRecord = env
            .storage()
            .persistent()
            .get(&agr_key)
            .ok_or(Error::AgreementNotFound)?;

        if record.status == AgreementStatus::Locked {
            return Err(Error::AgreementAlreadyLocked);
        }

        record.status = AgreementStatus::Locked;
        record.locked_at = env.ledger().timestamp();

        env.storage().persistent().set(&agr_key, &record);

        env.events().publish(
            (symbol_short!("agree"), symbol_short!("locked")),
            (tournament_id, version, record.agreement_hash.clone()),
        );

        Ok(record)
    }

    /// Retrieve a specific prize agreement version.
    pub fn get_prize_agreement(
        env: Env,
        tournament_id: u64,
        version: u32,
    ) -> Result<PrizeAgreementRecord, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::PrizeAgreement(tournament_id, version))
            .ok_or(Error::AgreementNotFound)
    }

    /// Check if the specified prize agreement version is locked.
    pub fn is_agreement_locked(env: Env, tournament_id: u64, version: u32) -> bool {
        match Self::get_prize_agreement(env, tournament_id, version) {
            Ok(agr) => agr.status == AgreementStatus::Locked,
            Err(_) => false,
        }
    }

    /// Query tournament details.
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
