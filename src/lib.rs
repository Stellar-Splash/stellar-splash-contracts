#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env,
    String, Vec,
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
    SettlementAlreadyExists = 23,
    SettlementNotFound = 24,
    SettlementAlreadyExecuted = 25,
    AgreementHashMismatch = 26,
    RankingHashMismatch = 27,
    AllocationExceedsPool = 28,
    EmptyAllocations = 29,
    ContractPaused = 30,
    TournamentNotEnded = 31,
    TitleTooLong = 32,
    GameIdTooLong = 33,
    InvalidTreasuryFeeBps = 34,
    TournamentDurationTooShort = 35,
    InvalidMaxParticipants = 36,
    AssetNotWhitelisted = 37,
    DisputeWindowNotElapsed = 38,
    DisputeAlreadySubmitted = 39,
    TournamentDisputed = 40,
    DisputeWindowElapsed = 41,
    EntryFeeAlreadyPaid = 42,
}

/// Maximum permissible byte length for title and game_id strings (storage safety limit)
pub const MAX_STRING_LENGTH: u32 = 128;
/// Maximum allowable treasury fee in basis points (1,000 bps = 10.00%)
pub const MAX_TREASURY_FEE_BPS: u32 = 1_000;
/// Minimum allowable tournament duration in seconds (60 seconds)
pub const MIN_TOURNAMENT_DURATION: u64 = 60;
/// Minimum allowable challenge / dispute window duration in seconds (300 seconds = 5 minutes)
pub const MIN_DISPUTE_WINDOW_SECONDS: u64 = 300;
/// Maximum permissible participant limit per tournament to protect storage scaling
pub const MAX_PARTICIPANTS_LIMIT: u32 = 10_000;
/// Ledger threshold (~1 day at 5 seconds per ledger)
pub const STORAGE_BUMP_THRESHOLD: u32 = 17_280;
/// Ledger extension target (~30 days at 5 seconds per ledger)
pub const STORAGE_LIFETIME_BUMP: u32 = 518_400;
/// Minimum allowable tournament prize pool amount (1 XLM = 10,000,000 Stroops)
pub const MIN_PRIZE_AMOUNT: i128 = 10_000_000;

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
pub struct ContractMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub protocol_version: u32,
    pub min_prize_amount: i128,
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
pub struct TournamentDetails {
    pub tournament_id: u64,
    pub state: TournamentState,
    pub target_prize_amount: i128,
    pub current_funded_amount: i128,
    pub current_participants: u32,
    pub max_participants: u32,
    pub start_time: u64,
    pub end_time: u64,
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
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SettlementStatus {
    Authorized = 0,
    Settled = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipientAllocation {
    pub recipient: Address,
    pub rank: u32,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementInput {
    pub tournament_id: u64,
    pub agreement_version: u32,
    pub agreement_hash: BytesN<32>,
    pub ranking_version: u32,
    pub ranking_hash: BytesN<32>,
    pub settlement_hash: BytesN<32>,
    pub allocations: Vec<RecipientAllocation>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementRecord {
    pub tournament_id: u64,
    pub agreement_version: u32,
    pub agreement_hash: BytesN<32>,
    pub ranking_version: u32,
    pub ranking_hash: BytesN<32>,
    pub settlement_hash: BytesN<32>,
    pub total_amount: i128,
    pub status: SettlementStatus,
    pub authorized_at: u64,
    pub settled_at: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DisputeStatus {
    Active = 0,
    Resolved = 1,
    Dismissed = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRecord {
    pub tournament_id: u64,
    pub challenger: Address,
    pub reason_code: u32,
    pub submitted_at: u64,
    pub status: DisputeStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeWindowInfo {
    pub end_timestamp: u64,
    pub is_elapsed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentCapacityInfo {
    pub current_participants: u32,
    pub max_participants: u32,
    pub is_full: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryFeeConfig {
    pub fee_asset: Address,
    pub fee_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerStatsRecord {
    pub player: Address,
    pub tournaments_joined: u32,
    pub tournaments_won: u32,
    pub total_score: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractStats {
    pub total_tournaments: u64,
    pub total_settled_amount: i128,
    pub total_volume: i128,
    pub is_paused: bool,
    pub treasury_fee_bps: u32,
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
    Settlement(u64),
    SettlementAllocations(u64),
    ParticipantList(u64),
    VaultFunder(u64),
    Paused,
    TreasuryAddress,
    TreasuryFeeBps,
    TotalSettledAmount,
    TotalVolume,
    TournamentMetadataUri(u64),
    WhitelistedAsset(Address),
    DisputeWindow(u64),
    DisputeRecord(u64, Address),
    TournamentDisputedFlag(u64),
    EntryFee(u64),
    TotalEntryFees(u64),
    PlayerEntryFeePaid(u64, Address),
    PlayerStats(Address),
    TournamentIdList,
    TournamentDispute(u64),
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
        env.storage()
            .instance()
            .set(&DataKey::TotalSettledAmount, &0i128);
        env.storage().instance().set(&DataKey::TotalVolume, &0i128);
        env.storage()
            .instance()
            .extend_ttl(STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(())
    }

    /// Retrieve the admin address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        env.storage()
            .instance()
            .extend_ttl(STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(admin)
    }

    /// Retrieve the contract deployer / root administrator address.
    pub fn get_contract_deployer(env: Env) -> Result<Address, Error> {
        Self::get_admin(env)
    }

    /// Explicitly extends instance storage TTL for contract metadata and admin records.
    pub fn extend_contract_instance_ttl(env: Env) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }
        env.storage()
            .instance()
            .extend_ttl(STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(())
    }

    /// Query immutable contract metadata descriptor.
    pub fn get_contract_metadata(env: Env) -> ContractMetadata {
        ContractMetadata {
            name: String::from_str(&env, "Stellar Splash Tournament Vault"),
            version: String::from_str(&env, "1.0.0"),
            description: String::from_str(
                &env,
                "Deterministic eSports escrow vault and settlement protocol on Soroban",
            ),
            protocol_version: 1,
            min_prize_amount: MIN_PRIZE_AMOUNT,
        }
    }

    /// Pause the contract operations (admin-only emergency control).
    pub fn pause(env: Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events()
            .publish((symbol_short!("circuit"), symbol_short!("paused")), ());
        Ok(())
    }

    /// Unpause the contract operations (admin-only).
    pub fn unpause(env: Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events()
            .publish((symbol_short!("circuit"), symbol_short!("unpaused")), ());
        Ok(())
    }

    /// Query whether contract operations are currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Configure protocol treasury address and optional basis-point fee (admin only).
    pub fn set_treasury(env: Env, treasury: Address, fee_bps: u32) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        if fee_bps > MAX_TREASURY_FEE_BPS {
            return Err(Error::InvalidTreasuryFeeBps);
        }

        env.storage()
            .instance()
            .set(&DataKey::TreasuryAddress, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::TreasuryFeeBps, &fee_bps);

        env.events().publish(
            (symbol_short!("treasury"), symbol_short!("set")),
            (treasury, fee_bps),
        );
        Ok(())
    }

    /// Retrieve the configured protocol treasury address (if set).
    pub fn get_treasury_address(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::TreasuryAddress)
    }

    /// Retrieve the configured protocol treasury fee in basis points.
    pub fn get_treasury_fee_bps(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::TreasuryFeeBps)
            .unwrap_or(0)
    }

    /// Computes the estimated protocol treasury fee for a specific tournament based on current funded amount.
    pub fn get_tournament_treasury_fee(env: Env, tournament_id: u64) -> Result<i128, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::TreasuryFeeBps)
            .unwrap_or(0);

        if fee_bps == 0 || info.current_funded_amount <= 0 {
            return Ok(0);
        }

        let fee = (info.current_funded_amount * (fee_bps as i128)) / 10_000;
        Ok(fee)
    }

    /// Whitelists or de-whitelists an asset for tournament prize pools (admin only).
    pub fn set_asset_whitelisted(env: Env, asset: Address, whitelisted: bool) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let key = DataKey::WhitelistedAsset(asset.clone());
        if whitelisted {
            env.storage().instance().set(&key, &true);
        } else {
            env.storage().instance().remove(&key);
        }

        env.events().publish(
            (symbol_short!("asset"), symbol_short!("wlist")),
            (asset, whitelisted),
        );
        Ok(())
    }

    /// Checks if a prize asset is whitelisted by the protocol admin.
    pub fn is_asset_whitelisted(env: Env, asset: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::WhitelistedAsset(asset))
            .unwrap_or(false)
    }

    /// Create a new tournament record.
    pub fn create_tournament(
        env: Env,
        creator: Address,
        config: TournamentConfig,
    ) -> Result<TournamentInfo, Error> {
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
        creator.require_auth();

        if config.target_prize_amount < MIN_PRIZE_AMOUNT {
            return Err(Error::InvalidAmount);
        }
        if config.end_time <= config.start_time {
            return Err(Error::InvalidTimeBounds);
        }
        if config.end_time < config.start_time + MIN_TOURNAMENT_DURATION {
            return Err(Error::TournamentDurationTooShort);
        }
        if config.max_participants == 0 || config.max_participants > MAX_PARTICIPANTS_LIMIT {
            return Err(Error::InvalidMaxParticipants);
        }
        if config.title.len() > MAX_STRING_LENGTH {
            return Err(Error::TitleTooLong);
        }
        if config.game_id.len() > MAX_STRING_LENGTH {
            return Err(Error::GameIdTooLong);
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
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TournamentCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TournamentCount, &(count + 1));

        let mut id_list: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::TournamentIdList)
            .unwrap_or(Vec::new(&env));
        id_list.push_back(config.tournament_id);
        env.storage()
            .instance()
            .set(&DataKey::TournamentIdList, &id_list);

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
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
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

        let funder_key = DataKey::VaultFunder(tournament_id);
        if !env.storage().persistent().has(&funder_key) {
            env.storage().persistent().set(&funder_key, &funder);
            env.storage().persistent().extend_ttl(
                &funder_key,
                STORAGE_BUMP_THRESHOLD,
                STORAGE_LIFETIME_BUMP,
            );
        }

        env.storage().persistent().set(&key, &tournament);

        let cur_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0);
        if let Some(new_vol) = cur_vol.checked_add(deposit_amount) {
            env.storage()
                .instance()
                .set(&DataKey::TotalVolume, &new_vol);
        }

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

    /// Fund prize pool using delegated allowance via SEP-41 transfer_from.
    pub fn fund_prize_pool_from(
        env: Env,
        spender: Address,
        funder: Address,
        tournament_id: u64,
        amount: i128,
    ) -> Result<TournamentInfo, Error> {
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
        spender.require_auth();

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
        client.transfer_from(
            &spender,
            &funder,
            &env.current_contract_address(),
            &deposit_amount,
        );

        tournament.current_funded_amount = tournament
            .current_funded_amount
            .checked_add(deposit_amount)
            .ok_or(Error::ArithmeticOverflow)?;

        if tournament.current_funded_amount >= tournament.target_prize_amount {
            tournament.state = TournamentState::Funded;
        }

        let funder_key = DataKey::VaultFunder(tournament_id);
        if !env.storage().persistent().has(&funder_key) {
            env.storage().persistent().set(&funder_key, &funder);
            env.storage().persistent().extend_ttl(
                &funder_key,
                STORAGE_BUMP_THRESHOLD,
                STORAGE_LIFETIME_BUMP,
            );
        }

        env.storage().persistent().set(&key, &tournament);

        let cur_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0);
        if let Some(new_vol) = cur_vol.checked_add(deposit_amount) {
            env.storage()
                .instance()
                .set(&DataKey::TotalVolume, &new_vol);
        }

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
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
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

        let stats_key = DataKey::PlayerStats(player.clone());
        let mut stats: PlayerStatsRecord =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(PlayerStatsRecord {
                    player: player.clone(),
                    tournaments_joined: 0,
                    tournaments_won: 0,
                    total_score: 0,
                });
        stats.tournaments_joined = stats
            .tournaments_joined
            .checked_add(1)
            .ok_or(Error::ArithmeticOverflow)?;
        env.storage().persistent().set(&stats_key, &stats);
        env.storage().persistent().extend_ttl(
            &stats_key,
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );

        let mut part_list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::ParticipantList(tournament_id))
            .unwrap_or(Vec::new(&env));
        part_list.push_back(player.clone());
        env.storage()
            .persistent()
            .set(&DataKey::ParticipantList(tournament_id), &part_list);

        tournament.current_participants = tournament
            .current_participants
            .checked_add(1)
            .ok_or(Error::ArithmeticOverflow)?;

        env.storage().persistent().set(&tourn_key, &tournament);

        if tournament.max_participants > 0
            && tournament.current_participants >= tournament.max_participants
        {
            env.events().publish(
                (symbol_short!("tourn"), symbol_short!("full")),
                (tournament_id, tournament.current_participants),
            );
        }

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

        let is_admin = match Self::get_admin(env.clone()) {
            Ok(adm) => adm == caller,
            Err(_) => false,
        };

        if tournament.creator != caller && !is_admin {
            return Err(Error::Unauthorized);
        }

        if tournament.state != TournamentState::InProgress
            && tournament.state != TournamentState::Open
        {
            return Err(Error::InvalidTournamentState);
        }

        let current_time = env.ledger().timestamp();
        let capacity_reached = tournament.max_participants > 0
            && tournament.current_participants >= tournament.max_participants;

        if !is_admin
            && tournament.end_time > 0
            && current_time < tournament.end_time
            && !capacity_reached
        {
            return Err(Error::TournamentNotEnded);
        }

        tournament.state = TournamentState::Completed;
        env.storage().persistent().set(&key, &tournament);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("done")),
            tournament_id,
        );

        Ok(tournament)
    }

    /// Cancel a tournament before in-progress/settlement and mark for refund.
    pub fn cancel_tournament(
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

        let is_admin = match Self::get_admin(env.clone()) {
            Ok(adm) => adm == caller,
            Err(_) => false,
        };

        if tournament.creator != caller && !is_admin {
            return Err(Error::Unauthorized);
        }

        if tournament.state == TournamentState::Completed
            || tournament.state == TournamentState::Cancelled
        {
            return Err(Error::InvalidTournamentState);
        }

        tournament.state = TournamentState::Cancelled;
        env.storage().persistent().set(&key, &tournament);
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("cancel")),
            (tournament_id, caller),
        );

        Ok(tournament)
    }

    /// Claim refund for a cancelled tournament.
    pub fn claim_refund(env: Env, funder: Address, tournament_id: u64) -> Result<i128, Error> {
        funder.require_auth();

        let key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.state != TournamentState::Cancelled {
            return Err(Error::InvalidTournamentState);
        }

        let refund_amount = tournament.current_funded_amount;
        if refund_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let is_admin = match Self::get_admin(env.clone()) {
            Ok(adm) => adm == funder,
            Err(_) => false,
        };

        if tournament.creator != funder && !is_admin {
            return Err(Error::Unauthorized);
        }

        let client = token::Client::new(&env, &tournament.prize_asset);
        client.transfer(&env.current_contract_address(), &funder, &refund_amount);

        tournament.current_funded_amount = 0;
        env.storage().persistent().set(&key, &tournament);
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        env.events().publish(
            (symbol_short!("prize"), symbol_short!("refund")),
            (tournament_id, funder, refund_amount),
        );

        Ok(refund_amount)
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

        if version > 1 {
            let prev_key = DataKey::PrizeAgreement(tournament_id, version - 1);
            if let Some(mut prev_record) = env
                .storage()
                .persistent()
                .get::<DataKey, PrizeAgreementRecord>(&prev_key)
            {
                if prev_record.status == AgreementStatus::PendingApproval
                    || prev_record.status == AgreementStatus::ReadyToLock
                {
                    prev_record.status = AgreementStatus::Superseded;
                    env.storage().persistent().set(&prev_key, &prev_record);
                    env.events().publish(
                        (symbol_short!("agree"), symbol_short!("supers")),
                        (tournament_id, version - 1, version),
                    );
                }
            }
        }

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
        env.storage()
            .persistent()
            .set(&DataKey::ActiveAgreementVersion(tournament_id), &version);

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

    /// Retrieve the lifecycle status of a specific prize agreement version.
    pub fn get_agreement_status(
        env: Env,
        tournament_id: u64,
        version: u32,
    ) -> Result<AgreementStatus, Error> {
        let agr = Self::get_prize_agreement(env, tournament_id, version)?;
        Ok(agr.status)
    }

    /// Retrieve the currently active agreement version for a tournament.
    pub fn get_active_agreement_version(env: Env, tournament_id: u64) -> Result<u32, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::ActiveAgreementVersion(tournament_id))
            .ok_or(Error::AgreementNotFound)
    }

    /// Query tournament details.
    pub fn get_tournament(env: Env, tournament_id: u64) -> Result<TournamentInfo, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(info)
    }

    /// Explicitly extend persistent TTL for a tournament record to prevent archiving.
    pub fn extend_tournament_ttl(env: Env, tournament_id: u64) -> Result<(), Error> {
        let key = DataKey::Tournament(tournament_id);
        if !env.storage().persistent().has(&key) {
            return Err(Error::TournamentNotFound);
        }
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(())
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

    /// Retrieve the address that funded the prize pool for a tournament.
    pub fn get_vault_funder(env: Env, tournament_id: u64) -> Result<Address, Error> {
        let funder_key = DataKey::VaultFunder(tournament_id);
        env.storage()
            .persistent()
            .get(&funder_key)
            .ok_or(Error::TournamentNotFunded)
    }

    /// Lightweight view function returning the tournament state without heavy deserialization.
    pub fn get_tournament_status(env: Env, tournament_id: u64) -> Result<TournamentState, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;
        Ok(info.state)
    }

    /// View function returning current active registered participant count.
    pub fn get_participant_count(env: Env, tournament_id: u64) -> Result<u32, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;
        Ok(info.current_participants)
    }

    /// Retrieve summarized tournament status and operational parameters.
    pub fn get_tournament_details(
        env: Env,
        tournament_id: u64,
    ) -> Result<TournamentDetails, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;
        Ok(TournamentDetails {
            tournament_id: info.tournament_id,
            state: info.state,
            target_prize_amount: info.target_prize_amount,
            current_funded_amount: info.current_funded_amount,
            current_participants: info.current_participants,
            max_participants: info.max_participants,
            start_time: info.start_time,
            end_time: info.end_time,
        })
    }

    /// View function returning immutable tournament configuration parameters.
    pub fn get_tournament_config(env: Env, tournament_id: u64) -> Result<TournamentConfig, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;
        Ok(TournamentConfig {
            tournament_id: info.tournament_id,
            title: info.title,
            game_id: info.game_id,
            prize_asset: info.prize_asset,
            target_prize_amount: info.target_prize_amount,
            max_participants: info.max_participants,
            start_time: info.start_time,
            end_time: info.end_time,
        })
    }

    /// Sets an external metadata URI (e.g. IPFS CID or Arweave transaction) for a tournament (creator or admin only).
    pub fn set_tournament_metadata_uri(
        env: Env,
        caller: Address,
        tournament_id: u64,
        uri: String,
    ) -> Result<(), Error> {
        caller.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::Unauthorized);
            }
        }

        let key = DataKey::TournamentMetadataUri(tournament_id);
        env.storage().persistent().set(&key, &uri);
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        env.events().publish(
            (symbol_short!("tourn"), symbol_short!("meta")),
            (tournament_id, uri),
        );
        Ok(())
    }

    /// Retrieves the metadata URI (IPFS/Arweave) for a tournament, if set.
    pub fn get_tournament_metadata_uri(env: Env, tournament_id: u64) -> Option<String> {
        env.storage()
            .persistent()
            .get(&DataKey::TournamentMetadataUri(tournament_id))
    }

    /// Retrieve paginated slice of registered tournament participants.
    pub fn get_participants_page(
        env: Env,
        tournament_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Address>, Error> {
        let key = DataKey::Tournament(tournament_id);
        if !env.storage().persistent().has(&key) {
            return Err(Error::TournamentNotFound);
        }

        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::ParticipantList(tournament_id))
            .unwrap_or(Vec::new(&env));

        let len = list.len();
        if start_index >= len {
            return Ok(Vec::new(&env));
        }

        let end_index = (start_index + limit).min(len);
        let mut page = Vec::new(&env);
        for i in start_index..end_index {
            page.push_back(list.get(i).unwrap());
        }

        Ok(page)
    }

    /// Returns capacity and registration status for a tournament.
    pub fn get_tournament_capacity_status(
        env: Env,
        tournament_id: u64,
    ) -> Result<TournamentCapacityInfo, Error> {
        let key = DataKey::Tournament(tournament_id);
        let info: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        let is_full = info.current_participants >= info.max_participants;
        Ok(TournamentCapacityInfo {
            current_participants: info.current_participants,
            max_participants: info.max_participants,
            is_full,
        })
    }

    // -------------------------------------------------------------
    // DETERMINISTIC SETTLEMENT LAYER
    // -------------------------------------------------------------

    /// Authorizes a deterministic settlement against locked agreement and finalized ranking.
    pub fn authorize_settlement(
        env: Env,
        caller: Address,
        input: SettlementInput,
    ) -> Result<SettlementRecord, Error> {
        caller.require_auth();

        let tourn_key = DataKey::Tournament(input.tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::Unauthorized);
            }
        }

        if tournament.state != TournamentState::Completed {
            return Err(Error::InvalidTournamentState);
        }

        // Validate prize agreement is locked and hash matches
        let agr_key = DataKey::PrizeAgreement(input.tournament_id, input.agreement_version);
        let agreement: PrizeAgreementRecord = env
            .storage()
            .persistent()
            .get(&agr_key)
            .ok_or(Error::AgreementNotFound)?;

        if agreement.status != AgreementStatus::Locked {
            return Err(Error::AgreementNotLocked);
        }
        if agreement.agreement_hash != input.agreement_hash {
            return Err(Error::AgreementHashMismatch);
        }

        // Validate final ranking exists, version matches, and ranking hash matches
        let rank_key = DataKey::Ranking(input.tournament_id);
        let ranking: RankingRecord = env
            .storage()
            .persistent()
            .get(&rank_key)
            .ok_or(Error::RankingNotFound)?;

        if ranking.ranking_version != input.ranking_version
            || ranking.ranking_hash != input.ranking_hash
        {
            return Err(Error::RankingHashMismatch);
        }

        // Settlement must not already exist
        let settle_key = DataKey::Settlement(input.tournament_id);
        if env.storage().persistent().has(&settle_key) {
            return Err(Error::SettlementAlreadyExists);
        }

        if input.allocations.is_empty() {
            return Err(Error::EmptyAllocations);
        }

        // Calculate total allocated and enforce invariant: <= current_funded_amount
        let mut total_allocated: i128 = 0;
        for alloc in input.allocations.iter() {
            if alloc.amount <= 0 {
                return Err(Error::InvalidAmount);
            }
            total_allocated = total_allocated
                .checked_add(alloc.amount)
                .ok_or(Error::ArithmeticOverflow)?;
        }

        if total_allocated > tournament.current_funded_amount {
            return Err(Error::AllocationExceedsPool);
        }

        let record = SettlementRecord {
            tournament_id: input.tournament_id,
            agreement_version: input.agreement_version,
            agreement_hash: input.agreement_hash,
            ranking_version: input.ranking_version,
            ranking_hash: input.ranking_hash,
            settlement_hash: input.settlement_hash.clone(),
            total_amount: total_allocated,
            status: SettlementStatus::Authorized,
            authorized_at: env.ledger().timestamp(),
            settled_at: 0,
        };

        env.storage().persistent().set(&settle_key, &record);
        env.storage().persistent().set(
            &DataKey::SettlementAllocations(input.tournament_id),
            &input.allocations,
        );

        env.events().publish(
            (symbol_short!("settle"), symbol_short!("auth")),
            (input.tournament_id, input.settlement_hash, total_allocated),
        );

        Ok(record)
    }

    /// Executes the multi-recipient settlement from the vault.
    pub fn execute_settlement(
        env: Env,
        caller: Address,
        tournament_id: u64,
    ) -> Result<SettlementRecord, Error> {
        caller.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let mut tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::Unauthorized);
            }
        }

        if !Self::is_dispute_window_elapsed(env.clone(), tournament_id) {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::DisputeWindowNotElapsed);
            }
        }

        if Self::is_tournament_disputed(env.clone(), tournament_id) {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::TournamentDisputed);
            }
        }

        let settle_key = DataKey::Settlement(tournament_id);
        let mut record: SettlementRecord = env
            .storage()
            .persistent()
            .get(&settle_key)
            .ok_or(Error::SettlementNotFound)?;

        if record.status == SettlementStatus::Settled {
            return Err(Error::SettlementAlreadyExecuted);
        }

        let alloc_key = DataKey::SettlementAllocations(tournament_id);
        let allocations: Vec<RecipientAllocation> = env
            .storage()
            .persistent()
            .get(&alloc_key)
            .ok_or(Error::SettlementNotFound)?;

        // Apply state effects BEFORE external interactions (Checks-Effects-Interactions)
        record.status = SettlementStatus::Settled;
        record.settled_at = env.ledger().timestamp();
        env.storage().persistent().set(&settle_key, &record);

        tournament.current_funded_amount = tournament
            .current_funded_amount
            .checked_sub(record.total_amount)
            .ok_or(Error::ArithmeticOverflow)?;
        env.storage().persistent().set(&tourn_key, &tournament);

        let total_settled_key = DataKey::TotalSettledAmount;
        let prev_settled: i128 = env
            .storage()
            .instance()
            .get(&total_settled_key)
            .unwrap_or(0i128);
        let new_settled = prev_settled
            .checked_add(record.total_amount)
            .ok_or(Error::ArithmeticOverflow)?;
        env.storage()
            .instance()
            .set(&total_settled_key, &new_settled);

        let token_client = token::Client::new(&env, &tournament.prize_asset);
        let contract_address = env.current_contract_address();

        for alloc in allocations.iter() {
            token_client.transfer(&contract_address, &alloc.recipient, &alloc.amount);
            env.events().publish(
                (symbol_short!("payout"), symbol_short!("recip")),
                (tournament_id, alloc.recipient, alloc.amount),
            );
        }

        env.events().publish(
            (symbol_short!("settle"), symbol_short!("done")),
            (tournament_id, record.total_amount, allocations.len()),
        );

        Ok(record)
    }

    /// Query the settlement record for a tournament.
    pub fn get_settlement(env: Env, tournament_id: u64) -> Result<SettlementRecord, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Settlement(tournament_id))
            .ok_or(Error::SettlementNotFound)
    }

    /// Query the settlement allocations for a tournament.
    pub fn get_settlement_allocations(
        env: Env,
        tournament_id: u64,
    ) -> Result<Vec<RecipientAllocation>, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::SettlementAllocations(tournament_id))
            .ok_or(Error::SettlementNotFound)
    }

    /// Check if a tournament has been settled.
    pub fn is_settled(env: Env, tournament_id: u64) -> bool {
        match Self::get_settlement(env, tournament_id) {
            Ok(s) => s.status == SettlementStatus::Settled,
            Err(_) => false,
        }
    }

    /// Returns the disbursed prize amount for a settled tournament (0 if not settled).
    pub fn get_settled_amount(env: Env, tournament_id: u64) -> i128 {
        match Self::get_settlement(env, tournament_id) {
            Ok(s) => {
                if s.status == SettlementStatus::Settled {
                    s.total_amount
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    /// Returns the settlement status for a tournament, or None if no settlement was created.
    pub fn get_tournament_settlement_status(
        env: Env,
        tournament_id: u64,
    ) -> Option<SettlementStatus> {
        let record = env
            .storage()
            .persistent()
            .get::<_, SettlementRecord>(&DataKey::Settlement(tournament_id))?;
        Some(record.status)
    }

    /// Returns the cumulative prize amount settled across all tournaments in the vault.
    pub fn get_total_settled_amount(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSettledAmount)
            .unwrap_or(0i128)
    }

    /// Returns the cumulative prize pool volume deposited across all tournaments in the vault.
    pub fn get_total_volume(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0i128)
    }

    /// Opens a time-locked dispute/challenge window for an authorized tournament (creator or admin only).
    pub fn open_dispute_window(
        env: Env,
        caller: Address,
        tournament_id: u64,
        duration_seconds: u64,
    ) -> Result<u64, Error> {
        caller.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != caller {
            let admin = Self::get_admin(env.clone())?;
            if admin != caller {
                return Err(Error::Unauthorized);
            }
        }

        if duration_seconds < MIN_DISPUTE_WINDOW_SECONDS {
            return Err(Error::TournamentDurationTooShort);
        }

        let end_timestamp = env
            .ledger()
            .timestamp()
            .checked_add(duration_seconds)
            .ok_or(Error::ArithmeticOverflow)?;

        let key = DataKey::DisputeWindow(tournament_id);
        env.storage().persistent().set(&key, &end_timestamp);
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        env.events().publish(
            (symbol_short!("dispute"), symbol_short!("open")),
            (tournament_id, end_timestamp),
        );

        Ok(end_timestamp)
    }

    /// Extends an active or configured dispute window (admin only).
    pub fn extend_dispute_window(
        env: Env,
        admin: Address,
        tournament_id: u64,
        extension_seconds: u64,
    ) -> Result<u64, Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let key = DataKey::DisputeWindow(tournament_id);
        let current_end: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        let new_end = current_end
            .checked_add(extension_seconds)
            .ok_or(Error::ArithmeticOverflow)?;

        env.storage().persistent().set(&key, &new_end);
        env.storage()
            .persistent()
            .extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);

        env.events().publish(
            (symbol_short!("dispute"), symbol_short!("extend")),
            (tournament_id, new_end),
        );

        Ok(new_end)
    }

    /// Retrieves the dispute window end timestamp if configured.
    pub fn get_dispute_window_end(env: Env, tournament_id: u64) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeWindow(tournament_id))
    }

    /// Checks if the dispute window has elapsed (or if none was configured).
    pub fn is_dispute_window_elapsed(env: Env, tournament_id: u64) -> bool {
        match Self::get_dispute_window_end(env.clone(), tournament_id) {
            Some(end_ts) => env.ledger().timestamp() >= end_ts,
            None => true,
        }
    }

    /// Returns structured dispute window info (end_timestamp, is_elapsed) if configured.
    pub fn get_tournament_dispute_window(
        env: Env,
        tournament_id: u64,
    ) -> Option<DisputeWindowInfo> {
        let end_ts = Self::get_dispute_window_end(env.clone(), tournament_id)?;
        let is_elapsed = env.ledger().timestamp() >= end_ts;
        Some(DisputeWindowInfo {
            end_timestamp: end_ts,
            is_elapsed,
        })
    }

    /// Submits a formal on-chain dispute challenge during an active dispute window.
    pub fn submit_dispute(
        env: Env,
        challenger: Address,
        tournament_id: u64,
        reason_code: u32,
    ) -> Result<DisputeRecord, Error> {
        challenger.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        if !env.storage().persistent().has(&tourn_key) {
            return Err(Error::TournamentNotFound);
        }

        // Verify dispute window is configured and has not elapsed
        if Self::get_dispute_window_end(env.clone(), tournament_id).is_none()
            || Self::is_dispute_window_elapsed(env.clone(), tournament_id)
        {
            return Err(Error::DisputeWindowElapsed);
        }

        let dispute_key = DataKey::DisputeRecord(tournament_id, challenger.clone());
        if env.storage().persistent().has(&dispute_key) {
            return Err(Error::DisputeAlreadySubmitted);
        }

        let record = DisputeRecord {
            tournament_id,
            challenger: challenger.clone(),
            reason_code,
            submitted_at: env.ledger().timestamp(),
            status: DisputeStatus::Active,
        };

        env.storage().persistent().set(&dispute_key, &record);
        env.storage()
            .persistent()
            .set(&DataKey::TournamentDispute(tournament_id), &record);
        env.storage()
            .persistent()
            .set(&DataKey::TournamentDisputedFlag(tournament_id), &true);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );
        env.storage().persistent().extend_ttl(
            &DataKey::TournamentDispute(tournament_id),
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );

        env.events().publish(
            (symbol_short!("dispute"), symbol_short!("filed")),
            (tournament_id, challenger, reason_code),
        );

        Ok(record)
    }

    /// Checks if a tournament is currently marked as disputed.
    pub fn is_tournament_disputed(env: Env, tournament_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::TournamentDisputedFlag(tournament_id))
            .unwrap_or(false)
    }

    /// Checks whether an on-chain dispute record exists and is in Active status.
    pub fn is_dispute_active(env: Env, tournament_id: u64) -> bool {
        match Self::get_dispute_status(env, tournament_id) {
            Some(d) => d.status == DisputeStatus::Active,
            None => false,
        }
    }

    /// Resolves an active dispute, clearing the hold on settlement (admin only).
    pub fn resolve_dispute(env: Env, admin: Address, tournament_id: u64) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&DataKey::TournamentDisputedFlag(tournament_id), &false);

        let t_dispute_key = DataKey::TournamentDispute(tournament_id);
        if let Some(mut dispute) = env
            .storage()
            .persistent()
            .get::<_, DisputeRecord>(&t_dispute_key)
        {
            dispute.status = DisputeStatus::Resolved;
            env.storage().persistent().set(&t_dispute_key, &dispute);
        }

        env.events().publish(
            (symbol_short!("dispute"), symbol_short!("reslvd")),
            tournament_id,
        );
        Ok(())
    }

    /// Dismisses an active dispute as unsubstantiated, clearing the hold on settlement (admin only).
    pub fn dismiss_dispute(env: Env, admin: Address, tournament_id: u64) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&DataKey::TournamentDisputedFlag(tournament_id), &false);

        let t_dispute_key = DataKey::TournamentDispute(tournament_id);
        if let Some(mut dispute) = env
            .storage()
            .persistent()
            .get::<_, DisputeRecord>(&t_dispute_key)
        {
            dispute.status = DisputeStatus::Dismissed;
            env.storage().persistent().set(&t_dispute_key, &dispute);
        }

        env.events().publish(
            (symbol_short!("dispute"), symbol_short!("dismis")),
            tournament_id,
        );
        Ok(())
    }

    /// Verifies and returns the token balance held in this contract instance's vault.
    pub fn get_contract_token_balance(env: Env, token_address: Address) -> i128 {
        let token_client = token::Client::new(&env, &token_address);
        token_client.balance(&env.current_contract_address())
    }

    /// Upgrades the contract's executable WASM bytecode (admin only).
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.events().publish(
            (symbol_short!("contract"), symbol_short!("upgrade")),
            (admin, new_wasm_hash.clone()),
        );

        env.deployer().update_current_contract_wasm(new_wasm_hash);
        env.storage()
            .instance()
            .extend_ttl(STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
        Ok(())
    }

    /// Configures optional entry fee asset and amount required for players to join (creator only).
    pub fn set_entry_fee(
        env: Env,
        creator: Address,
        tournament_id: u64,
        fee_asset: Address,
        fee_amount: i128,
    ) -> Result<(), Error> {
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
        creator.require_auth();

        if fee_amount < 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.creator != creator {
            return Err(Error::Unauthorized);
        }

        if tournament.state != TournamentState::Created
            && tournament.state != TournamentState::Funding
            && tournament.state != TournamentState::Funded
        {
            return Err(Error::InvalidTournamentState);
        }

        let fee_key = DataKey::EntryFee(tournament_id);
        let config = EntryFeeConfig {
            fee_asset: fee_asset.clone(),
            fee_amount,
        };

        env.storage().persistent().set(&fee_key, &config);
        env.storage().persistent().extend_ttl(
            &fee_key,
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );

        env.events().publish(
            (symbol_short!("fee"), symbol_short!("set")),
            (tournament_id, fee_asset, fee_amount),
        );

        Ok(())
    }

    /// Allows a registered or joining player to deposit the required entry fee into the escrow vault.
    pub fn pay_entry_fee(env: Env, player: Address, tournament_id: u64) -> Result<(), Error> {
        if Self::is_paused(env.clone()) {
            return Err(Error::ContractPaused);
        }
        player.require_auth();

        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        if tournament.state != TournamentState::Open {
            return Err(Error::TournamentNotOpen);
        }

        let fee_key = DataKey::EntryFee(tournament_id);
        let entry_fee = env
            .storage()
            .persistent()
            .get::<_, EntryFeeConfig>(&fee_key)
            .unwrap_or(EntryFeeConfig {
                fee_asset: tournament.prize_asset.clone(),
                fee_amount: 0,
            });

        let paid_key = DataKey::PlayerEntryFeePaid(tournament_id, player.clone());
        if env.storage().persistent().has(&paid_key) {
            return Err(Error::EntryFeeAlreadyPaid);
        }

        if entry_fee.fee_amount > 0 {
            let client = token::Client::new(&env, &entry_fee.fee_asset);
            client.transfer(
                &player,
                &env.current_contract_address(),
                &entry_fee.fee_amount,
            );

            let total_fees_key = DataKey::TotalEntryFees(tournament_id);
            let current_total: i128 = env.storage().persistent().get(&total_fees_key).unwrap_or(0);
            let new_total = current_total
                .checked_add(entry_fee.fee_amount)
                .ok_or(Error::ArithmeticOverflow)?;
            env.storage().persistent().set(&total_fees_key, &new_total);
            env.storage().persistent().extend_ttl(
                &total_fees_key,
                STORAGE_BUMP_THRESHOLD,
                STORAGE_LIFETIME_BUMP,
            );

            let cur_vol: i128 = env
                .storage()
                .instance()
                .get(&DataKey::TotalVolume)
                .unwrap_or(0);
            if let Some(new_vol) = cur_vol.checked_add(entry_fee.fee_amount) {
                env.storage()
                    .instance()
                    .set(&DataKey::TotalVolume, &new_vol);
            }
        }

        env.storage().persistent().set(&paid_key, &true);
        env.storage().persistent().extend_ttl(
            &paid_key,
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );

        env.events().publish(
            (symbol_short!("fee"), symbol_short!("paid")),
            (tournament_id, player, entry_fee.fee_amount),
        );

        Ok(())
    }

    /// Retrieves the entry fee configuration (fee asset and amount) for a tournament.
    pub fn get_entry_fee(env: Env, tournament_id: u64) -> Result<EntryFeeConfig, Error> {
        let tourn_key = DataKey::Tournament(tournament_id);
        let tournament: TournamentInfo = env
            .storage()
            .persistent()
            .get(&tourn_key)
            .ok_or(Error::TournamentNotFound)?;

        let fee_key = DataKey::EntryFee(tournament_id);
        let entry_fee = env
            .storage()
            .persistent()
            .get::<_, EntryFeeConfig>(&fee_key)
            .unwrap_or(EntryFeeConfig {
                fee_asset: tournament.prize_asset,
                fee_amount: 0,
            });

        Ok(entry_fee)
    }

    /// Retrieves total entry fees accumulated in escrow for a tournament.
    pub fn get_total_entry_fees(env: Env, tournament_id: u64) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::TotalEntryFees(tournament_id))
            .unwrap_or(0)
    }

    /// Checks if a player has deposited the entry fee for a tournament.
    pub fn has_paid_entry_fee(env: Env, tournament_id: u64, player: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::PlayerEntryFeePaid(tournament_id, player))
            .unwrap_or(false)
    }

    /// Records an on-chain tournament victory for a player (admin only).
    pub fn increment_player_wins(env: Env, admin: Address, player: Address) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let stats_key = DataKey::PlayerStats(player.clone());
        let mut stats: PlayerStatsRecord =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(PlayerStatsRecord {
                    player: player.clone(),
                    tournaments_joined: 0,
                    tournaments_won: 0,
                    total_score: 0,
                });

        stats.tournaments_won = stats
            .tournaments_won
            .checked_add(1)
            .ok_or(Error::ArithmeticOverflow)?;

        env.storage().persistent().set(&stats_key, &stats);
        env.storage().persistent().extend_ttl(
            &stats_key,
            STORAGE_BUMP_THRESHOLD,
            STORAGE_LIFETIME_BUMP,
        );

        env.events().publish(
            (symbol_short!("player"), symbol_short!("win")),
            (player, stats.tournaments_won),
        );

        Ok(())
    }

    /// Retrieves on-chain career statistics for a player address.
    pub fn get_player_stats(env: Env, player: Address) -> PlayerStatsRecord {
        let stats_key = DataKey::PlayerStats(player.clone());
        env.storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(PlayerStatsRecord {
                player,
                tournaments_joined: 0,
                tournaments_won: 0,
                total_score: 0,
            })
    }

    /// Returns vector of tournament IDs that are currently active (not Completed or Cancelled).
    pub fn get_active_tournaments(env: Env) -> Vec<u64> {
        let id_list: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::TournamentIdList)
            .unwrap_or(Vec::new(&env));

        let mut active = Vec::new(&env);
        for id in id_list.iter() {
            if let Some(t) = env
                .storage()
                .persistent()
                .get::<_, TournamentInfo>(&DataKey::Tournament(id))
            {
                if t.state != TournamentState::Completed && t.state != TournamentState::Cancelled {
                    active.push_back(id);
                }
            }
        }
        active
    }

    /// Returns the recorded dispute status and metadata for a tournament, if one was ever filed.
    pub fn get_dispute_status(env: Env, tournament_id: u64) -> Option<DisputeRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::TournamentDispute(tournament_id))
    }

    /// Returns global aggregated statistics and contract state parameters.
    pub fn get_contract_stats(env: Env) -> ContractStats {
        let total_tournaments = env
            .storage()
            .instance()
            .get(&DataKey::TournamentCount)
            .unwrap_or(0u64);
        let total_settled_amount = env
            .storage()
            .instance()
            .get(&DataKey::TotalSettledAmount)
            .unwrap_or(0i128);
        let total_volume = env
            .storage()
            .instance()
            .get(&DataKey::TotalVolume)
            .unwrap_or(0i128);
        let is_paused = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        let treasury_fee_bps = env
            .storage()
            .instance()
            .get(&DataKey::TreasuryFeeBps)
            .unwrap_or(0u32);

        ContractStats {
            total_tournaments,
            total_settled_amount,
            total_volume,
            is_paused,
            treasury_fee_bps,
        }
    }

    /// Returns the active contract/protocol version (Protocol 21).
    pub fn get_contract_version(_env: Env) -> u32 {
        21
    }
}

#[cfg(test)]
mod test;
