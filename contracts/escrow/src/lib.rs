#![no_std]
#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, symbol_short, token, Address, BytesN, Env, String, Symbol, Vec};

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

pub use errors::EscrowError;
pub use events::EscrowEvents;
pub use types::*;

/// Maximum allowed platform fee: 10.00% (1,000 bps)
pub const MAX_FEE_BPS: u32 = 1_000;
/// Maximum supported milestones per escrow to ensure bounded gas execution
pub const MAX_MILESTONES: u32 = 25;

fn validate_fee(fee_bps: u32) -> Result<(), EscrowError> {
    if fee_bps > MAX_FEE_BPS {
        return Err(EscrowError::InvalidFee);
    }
    Ok(())
}

fn ensure_not_paused(env: &Env) -> Result<(), EscrowError> {
    if env.storage().instance().get(&DataKey::Paused).unwrap_or(false) {
        return Err(EscrowError::ContractPaused);
    }
    Ok(())
}

fn extend_storage_ttl(env: &Env, job_id: &String) {
    env.storage().instance().extend_ttl(100_000, 200_000);
    let escrow_key = DataKey::Escrow(job_id.clone());
    if env.storage().persistent().has(&escrow_key) {
        env.storage().persistent().extend_ttl(&escrow_key, 100_000, 200_000);
    }
    let milestones_key = DataKey::Milestones(job_id.clone());
    if env.storage().persistent().has(&milestones_key) {
        env.storage().persistent().extend_ttl(&milestones_key, 100_000, 200_000);
    }
}

/// Minimum allowed escrow timeout duration from creation time (prevents clock skew exploits)
pub const MIN_TIMEOUT_DURATION: u64 = 60;

fn next_receipt_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ReceiptCounter)
        .unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&DataKey::ReceiptCounter, &next);
    next
}

fn enforce_action_throttle(env: &Env, action: Symbol, min_interval_seconds: u64) -> Result<(), EscrowError> {
    let key = DataKey::AdminActionThrottle(action);
    let now = env.ledger().timestamp();
    if let Some(last_time) = env.storage().instance().get::<_, u64>(&key) {
        if now < last_time + min_interval_seconds {
            return Err(EscrowError::RateLimitExceeded);
        }
    }
    env.storage().instance().set(&key, &now);
    Ok(())
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Initialize the escrow platform with admin and treasury configurations.
    pub fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        fee_bps: u32,
    ) -> Result<(), EscrowError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(EscrowError::AlreadyInitialized);
        }
        admin.require_auth();
        validate_fee(fee_bps)?;

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().extend_ttl(100_000, 200_000);

        Ok(())
    }

    /// Pause all state-mutating contract operations (admin only).
    pub fn pause(env: Env) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        EscrowEvents::pause(&env, admin);
        Ok(())
    }

    /// Resume paused contract operations (admin only).
    pub fn unpause(env: Env) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        EscrowEvents::unpause(&env, admin);
        Ok(())
    }

    /// Query whether the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    /// Request emergency freeze with mandatory notice period (admin only).
    pub fn request_emergency_freeze(env: Env, notice_period_seconds: u64) -> Result<u64, EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        let period = if notice_period_seconds < 86_400 {
            86_400
        } else {
            notice_period_seconds
        };

        let now = env.ledger().timestamp();
        let unlock_timestamp = now + period;

        env.storage().instance().set(&DataKey::EmergencyFreezeActive, &true);
        env.storage().instance().set(&DataKey::EmergencyFreezeUnlock, &unlock_timestamp);

        EscrowEvents::emergency_freeze_initiated(&env, admin, unlock_timestamp);
        Ok(unlock_timestamp)
    }

    /// Query emergency freeze status and unlock timestamp.
    pub fn get_emergency_freeze_status(env: Env) -> (bool, u64) {
        let active = env.storage().instance().get(&DataKey::EmergencyFreezeActive).unwrap_or(false);
        let unlock_timestamp = env.storage().instance().get(&DataKey::EmergencyFreezeUnlock).unwrap_or(0);
        (active, unlock_timestamp)
    }

    /// Execute emergency unfreeze once notice period has elapsed (admin only).
    pub fn execute_emergency_unfreeze(env: Env) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        let is_active: bool = env.storage().instance().get(&DataKey::EmergencyFreezeActive).unwrap_or(false);
        if !is_active {
            return Ok(());
        }

        let unlock_timestamp: u64 = env.storage().instance().get(&DataKey::EmergencyFreezeUnlock).unwrap_or(0);
        let now = env.ledger().timestamp();
        if now < unlock_timestamp {
            return Err(EscrowError::FreezePeriodNotExpired);
        }

        env.storage().instance().set(&DataKey::EmergencyFreezeActive, &false);
        env.storage().instance().remove(&DataKey::EmergencyFreezeUnlock);

        EscrowEvents::emergency_freeze_lifted(&env, admin);
        Ok(())
    }

    /// Update platform fee basis points (admin only). 100 bps = 1.00%.
    pub fn set_fee_bps(env: Env, fee_bps: u32) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();
        validate_fee(fee_bps)?;

        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
        Ok(())
    }

    /// Update platform treasury recipient address (admin only, throttled to 300s minimum interval).
    pub fn set_treasury(env: Env, new_treasury: Address) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        enforce_action_throttle(&env, symbol_short!("treasury"), 300)?;

        env.storage().instance().set(&DataKey::Treasury, &new_treasury);
        EscrowEvents::set_treasury(&env, new_treasury);
        Ok(())
    }

    /// Transfer contract admin ownership to a new address (current admin only).
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        EscrowEvents::transfer_admin(&env, new_admin);
        Ok(())
    }

    /// Register an arbitrator by depositing a staking bond.
    pub fn register_arbitrator(
        env: Env,
        arbitrator: Address,
        token: Address,
        bond_amount: i128,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        arbitrator.require_auth();

        if bond_amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let bond_key = DataKey::ArbitratorBond(arbitrator.clone());
        let current_bond: i128 = env.storage().persistent().get(&bond_key).unwrap_or(0);

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&arbitrator, &env.current_contract_address(), &bond_amount);

        let new_total = current_bond + bond_amount;
        env.storage().persistent().set(&bond_key, &new_total);

        EscrowEvents::arbitrator_registered(&env, arbitrator, new_total);

        Ok(())
    }

    /// Withdraw a portion or all of an arbitrator's bonded stake.
    pub fn withdraw_arbitrator_bond(
        env: Env,
        arbitrator: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        arbitrator.require_auth();

        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let bond_key = DataKey::ArbitratorBond(arbitrator.clone());
        let current_bond: i128 = env.storage().persistent().get(&bond_key).unwrap_or(0);

        if amount > current_bond {
            return Err(EscrowError::InvalidAmount);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &arbitrator, &amount);

        let new_total = current_bond - amount;
        if new_total == 0 {
            env.storage().persistent().remove(&bond_key);
        } else {
            env.storage().persistent().set(&bond_key, &new_total);
        }

        EscrowEvents::arbitrator_bond_withdrawn(&env, arbitrator, amount, new_total);

        Ok(())
    }

    /// Slash an arbitrator's bond for proven misconduct and transfer penalty to recipient (admin only).
    pub fn slash_arbitrator_bond(
        env: Env,
        admin: Address,
        arbitrator: Address,
        recipient: Address,
        token: Address,
        slash_amount: i128,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if stored_admin != admin {
            return Err(EscrowError::Unauthorized);
        }
        admin.require_auth();

        if slash_amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let bond_key = DataKey::ArbitratorBond(arbitrator.clone());
        let current_bond: i128 = env.storage().persistent().get(&bond_key).unwrap_or(0);

        if current_bond < slash_amount {
            return Err(EscrowError::InsufficientBond);
        }

        let new_total = current_bond - slash_amount;
        env.storage().persistent().set(&bond_key, &new_total);

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &recipient, &slash_amount);

        EscrowEvents::arbitrator_bond_slashed(&env, arbitrator, recipient, slash_amount);

        Ok(())
    }

    /// Create an escrow with optional milestones and lock funds from the client into contract custody.
    pub fn create_escrow(
        env: Env,
        job_id: String,
        client: Address,
        params: CreateEscrowParams,
        milestones: Vec<MilestoneInput>,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        if params.total_amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        if milestones.len() > MAX_MILESTONES {
            return Err(EscrowError::InvalidMilestoneCount);
        }

        let escrow_key = DataKey::Escrow(job_id.clone());
        if env.storage().persistent().has(&escrow_key) {
            return Err(EscrowError::EscrowAlreadyExists);
        }

        let now = env.ledger().timestamp();
        if params.timeout_timestamp < (now + MIN_TIMEOUT_DURATION) {
            return Err(EscrowError::TimeoutAlreadyExpired);
        }

        // Validate milestone percentage distribution
        let mut contract_milestones: Vec<Milestone> = Vec::new(&env);
        if milestones.is_empty() {
            // Default 100% single milestone
            contract_milestones.push_back(Milestone {
                id: 1,
                description: String::from_str(&env, "Full project delivery"),
                percentage: 100,
                amount: params.total_amount,
                approved: false,
            });
        } else {
            let mut total_pct: u32 = 0;
            let mut allocated_so_far: i128 = 0;
            let milestone_count = milestones.len();

            for (idx, m) in milestones.iter().enumerate() {
                if m.percentage == 0 || m.percentage > 100 {
                    return Err(EscrowError::InvalidMilestonePercentages);
                }
                if m.description.is_empty() || m.description.len() > 256 {
                    return Err(EscrowError::InvalidMilestoneData);
                }
                total_pct = total_pct
                    .checked_add(m.percentage)
                    .ok_or(EscrowError::InvalidMilestonePercentages)?;
                let is_last = (idx + 1) == (milestone_count as usize);

                let milestone_amount = if is_last {
                    params.total_amount - allocated_so_far
                } else {
                    (params.total_amount * (m.percentage as i128)) / 100
                };
                allocated_so_far += milestone_amount;

                contract_milestones.push_back(Milestone {
                    id: (idx + 1) as u32,
                    description: m.description,
                    percentage: m.percentage,
                    amount: milestone_amount,
                    approved: false,
                });
            }
            if total_pct != 100 {
                return Err(EscrowError::InvalidMilestonePercentages);
            }
        }

        // Transfer funds from client to contract custody
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &params.token);
        token_client.transfer(&client, &contract_address, &params.total_amount);

        let escrow = Escrow {
            job_id: job_id.clone(),
            client,
            freelancer: params.freelancer,
            token: params.token,
            total_amount: params.total_amount,
            released_amount: 0,
            status: EscrowStatus::Created,
            created_at: now,
            timeout_timestamp: params.timeout_timestamp,
            arbitrator: params.arbitrator,
        };

        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::Milestones(job_id.clone()), &contract_milestones);

        extend_storage_ttl(&env, &job_id);

        EscrowEvents::escrow_created(&env, job_id, escrow.total_amount);

        Ok(())
    }

    /// Freelancer indicates work has commenced, transitioning status from Created to InProgress.
    pub fn start_work(env: Env, job_id: String, freelancer: Address) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        freelancer.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.freelancer != freelancer {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        escrow.status = EscrowStatus::InProgress;
        env.storage().persistent().set(&escrow_key, &escrow);
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::escrow_started(&env, job_id);

        Ok(())
    }

    /// Freelancer registers a cryptographic proof hash (e.g. SHA-256 or IPFS CID) for a deliverable.
    pub fn submit_deliverable(
        env: Env,
        job_id: String,
        freelancer: Address,
        deliverable_hash: BytesN<32>,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        freelancer.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.freelancer != freelancer {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Deliverable(job_id.clone()), &deliverable_hash);
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::deliverable_submitted(&env, job_id, deliverable_hash);

        Ok(())
    }

    /// Freelancer registers a cryptographic proof hash for a specific milestone ID.
    pub fn submit_milestone_deliverable(
        env: Env,
        job_id: String,
        freelancer: Address,
        milestone_id: u32,
        deliverable_hash: BytesN<32>,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        freelancer.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.freelancer != freelancer {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestones_key = DataKey::Milestones(job_id.clone());
        let milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&milestones_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        let mut found = false;
        for ms in milestones.iter() {
            if ms.id == milestone_id {
                found = true;
                break;
            }
        }
        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }

        env.storage().persistent().set(
            &DataKey::MilestoneDeliverable(job_id.clone(), milestone_id),
            &deliverable_hash,
        );
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::milestone_deliverable_submitted(&env, job_id, milestone_id, deliverable_hash);

        Ok(())
    }

    /// Client approves a milestone, releasing pro-rata funds to the freelancer less treasury fee.
    pub fn release_milestone(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestones_key = DataKey::Milestones(job_id.clone());
        let milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&milestones_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        let mut milestone_found = false;
        let mut release_amount: i128 = 0;
        let mut all_completed = true;

        let mut updated_milestones: Vec<Milestone> = Vec::new(&env);
        for mut m in milestones.iter() {
            if m.id == milestone_id {
                if m.approved {
                    return Err(EscrowError::MilestoneAlreadyApproved);
                }
                m.approved = true;
                milestone_found = true;
                release_amount = m.amount;
            }
            if !m.approved {
                all_completed = false;
            }
            updated_milestones.push_back(m);
        }

        if !milestone_found {
            return Err(EscrowError::MilestoneNotFound);
        }

        // Checks & Effects: update state before external interactions (CEI pattern)
        escrow.released_amount += release_amount;
        if all_completed || escrow.released_amount >= escrow.total_amount {
            escrow.status = EscrowStatus::Completed;
        } else {
            escrow.status = EscrowStatus::InProgress;
        }

        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage()
            .persistent()
            .set(&milestones_key, &updated_milestones);
        extend_storage_ttl(&env, &job_id);

        // Interactions: external token settlement
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let treasury: Option<Address> = env.storage().instance().get(&DataKey::Treasury);

        let fee = release_amount
            .checked_mul(fee_bps as i128)
            .ok_or(EscrowError::InvalidAmount)?
            .checked_div(10_000)
            .ok_or(EscrowError::InvalidAmount)?;
        let freelancer_payout = release_amount
            .checked_sub(fee)
            .ok_or(EscrowError::InvalidAmount)?;

        let token_client = token::Client::new(&env, &escrow.token);

        if fee > 0 {
            if let Some(treasury_addr) = treasury {
                token_client.transfer(&env.current_contract_address(), &treasury_addr, &fee);
            }
        }

        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &freelancer_payout,
        );

        EscrowEvents::milestone_released(&env, job_id.clone(), milestone_id, release_amount);

        let receipt_id = next_receipt_id(&env);
        EscrowEvents::receipt_issued(
            &env,
            receipt_id,
            job_id,
            milestone_id,
            escrow.freelancer,
            release_amount,
            fee,
        );

        Ok(())
    }

    /// Client cancels the escrow and reclaims unreleased funds if work has not yet commenced.
    pub fn cancel_and_refund(env: Env, job_id: String, client: Address) -> Result<(), EscrowError> {
        client.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let remaining = escrow.total_amount - escrow.released_amount;
        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&escrow_key, &escrow);

        if remaining > 0 {
            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(&env.current_contract_address(), &escrow.client, &remaining);
            let receipt_id = next_receipt_id(&env);
            EscrowEvents::receipt_issued(
                &env,
                receipt_id,
                job_id.clone(),
                0,
                escrow.client.clone(),
                remaining,
                0,
            );
        }

        EscrowEvents::escrow_refunded(&env, job_id, remaining);

        Ok(())
    }

    /// Mutual cancellation and refund of remaining funds while InProgress (requires dual auth of client and freelancer).
    pub fn mutual_cancel(env: Env, job_id: String) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        escrow.client.require_auth();
        escrow.freelancer.require_auth();

        if escrow.status != EscrowStatus::InProgress {
            return Err(EscrowError::InvalidStatus);
        }

        let remaining = escrow.total_amount - escrow.released_amount;
        escrow.status = if escrow.released_amount > 0 {
            EscrowStatus::Completed
        } else {
            EscrowStatus::Refunded
        };
        env.storage().persistent().set(&escrow_key, &escrow);
        extend_storage_ttl(&env, &job_id);

        if remaining > 0 {
            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(&env.current_contract_address(), &escrow.client, &remaining);
        }

        EscrowEvents::mutual_cancelled(&env, job_id, remaining);

        Ok(())
    }

    /// Permissionless timeout refund: anyone can invoke this to return remaining funds to the client
    /// once the ledger timestamp exceeds the timeout deadline and the work is unfinished.
    pub fn claim_timeout_refund(env: Env, job_id: String) -> Result<(), EscrowError> {
        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        let now = env.ledger().timestamp();
        let is_standard_timeout = (escrow.status == EscrowStatus::Created
            || escrow.status == EscrowStatus::InProgress)
            && now >= escrow.timeout_timestamp;
        let is_dispute_timeout = escrow.status == EscrowStatus::Disputed
            && now >= (escrow.timeout_timestamp + 2_592_000);

        if !is_standard_timeout && !is_dispute_timeout {
            if escrow.status == EscrowStatus::Disputed {
                return Err(EscrowError::InvalidStatus);
            }
            if now < escrow.timeout_timestamp {
                return Err(EscrowError::TimeoutNotExpired);
            }
            return Err(EscrowError::InvalidStatus);
        }

        let remaining = escrow.total_amount - escrow.released_amount;
        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&escrow_key, &escrow);

        if remaining > 0 {
            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(&env.current_contract_address(), &escrow.client, &remaining);
            let receipt_id = next_receipt_id(&env);
            EscrowEvents::receipt_issued(
                &env,
                receipt_id,
                job_id.clone(),
                0,
                escrow.client.clone(),
                remaining,
                0,
            );
        }
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::timeout_refunded(&env, job_id, remaining);

        Ok(())
    }

    /// Raise a dispute on an active escrow (invoked by client or freelancer).
    pub fn raise_dispute(env: Env, job_id: String, caller: Address) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        caller.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if caller != escrow.client && caller != escrow.freelancer {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        escrow.status = EscrowStatus::Disputed;
        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::DisputeTimestamp(job_id.clone()), &env.ledger().timestamp());
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::dispute_raised(&env, job_id, caller);

        Ok(())
    }

    /// Submit cryptographic proof/evidence hash for an active dispute (client or freelancer).
    pub fn submit_dispute_evidence(
        env: Env,
        job_id: String,
        caller: Address,
        evidence_hash: BytesN<32>,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        caller.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if caller != escrow.client && caller != escrow.freelancer {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::Disputed {
            return Err(EscrowError::EscrowNotDisputed);
        }

        env.storage().persistent().set(
            &DataKey::DisputeEvidence(job_id.clone(), caller.clone()),
            &evidence_hash,
        );
        extend_storage_ttl(&env, &job_id);

        EscrowEvents::evidence_submitted(&env, job_id, caller, evidence_hash);

        Ok(())
    }

    /// Arbitrator resolves a disputed escrow by allocating remaining funds between client and freelancer.
    pub fn resolve_dispute(
        env: Env,
        job_id: String,
        arbitrator: Address,
        client_award: i128,
        freelancer_award: i128,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        arbitrator.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.arbitrator != arbitrator {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::Disputed {
            return Err(EscrowError::EscrowNotDisputed);
        }

        let cooldown: u64 = env.storage().instance().get(&DataKey::DisputeCooldown).unwrap_or(0);
        if cooldown > 0 {
            let dispute_time: u64 = env
                .storage()
                .persistent()
                .get(&DataKey::DisputeTimestamp(job_id.clone()))
                .unwrap_or(0);
            if env.ledger().timestamp() < dispute_time + cooldown {
                return Err(EscrowError::RateLimitExceeded);
            }
        }

        let remaining = escrow.total_amount - escrow.released_amount;
        if client_award < 0
            || freelancer_award < 0
            || (client_award + freelancer_award) != remaining
        {
            return Err(EscrowError::InvalidAmount);
        }

        Self::execute_dispute_resolution(&env, &job_id, client_award, freelancer_award)
    }

    /// Cast a vote in a multi-arbitrator 2-of-3 quorum panel.
    /// Returns true if 2-of-3 quorum was reached and dispute was settled; false if pending more votes.
    pub fn vote_dispute(
        env: Env,
        job_id: String,
        arbitrator: Address,
        client_award: i128,
        freelancer_award: i128,
    ) -> Result<bool, EscrowError> {
        ensure_not_paused(&env)?;
        arbitrator.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.status != EscrowStatus::Disputed {
            return Err(EscrowError::EscrowNotDisputed);
        }

        let cooldown: u64 = env.storage().instance().get(&DataKey::DisputeCooldown).unwrap_or(0);
        if cooldown > 0 {
            let dispute_time: u64 = env
                .storage()
                .persistent()
                .get(&DataKey::DisputeTimestamp(job_id.clone()))
                .unwrap_or(0);
            if env.ledger().timestamp() < dispute_time + cooldown {
                return Err(EscrowError::RateLimitExceeded);
            }
        }

        let remaining = escrow.total_amount - escrow.released_amount;
        if client_award < 0
            || freelancer_award < 0
            || (client_award + freelancer_award) != remaining
        {
            return Err(EscrowError::InvalidAmount);
        }

        let ballot_key = DataKey::DisputeBallot(job_id.clone());
        let mut votes: Vec<ArbitratorVote> = env
            .storage()
            .persistent()
            .get(&ballot_key)
            .unwrap_or_else(|| Vec::new(&env));

        for vote in votes.iter() {
            if vote.arbitrator == arbitrator {
                return Err(EscrowError::Unauthorized);
            }
        }

        votes.push_back(ArbitratorVote {
            arbitrator: arbitrator.clone(),
            client_award,
            freelancer_award,
        });
        env.storage().persistent().set(&ballot_key, &votes);
        extend_storage_ttl(&env, &job_id);

        let mut matching_count = 0;
        for vote in votes.iter() {
            if vote.client_award == client_award && vote.freelancer_award == freelancer_award {
                matching_count += 1;
            }
        }

        if matching_count >= 2 {
            Self::execute_dispute_resolution(&env, &job_id, client_award, freelancer_award)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn execute_dispute_resolution(
        env: &Env,
        job_id: &String,
        client_award: i128,
        freelancer_award: i128,
    ) -> Result<(), EscrowError> {
        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        escrow.released_amount += freelancer_award;
        escrow.status = EscrowStatus::Completed;
        env.storage().persistent().set(&escrow_key, &escrow);
        extend_storage_ttl(env, job_id);

        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let treasury: Option<Address> = env.storage().instance().get(&DataKey::Treasury);

        let fee = freelancer_award
            .checked_mul(fee_bps as i128)
            .ok_or(EscrowError::InvalidAmount)?
            .checked_div(10_000)
            .ok_or(EscrowError::InvalidAmount)?;
        let freelancer_payout = freelancer_award
            .checked_sub(fee)
            .ok_or(EscrowError::InvalidAmount)?;

        let token_client = token::Client::new(env, &escrow.token);

        if client_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.client,
                &client_award,
            );
            let client_receipt_id = next_receipt_id(env);
            EscrowEvents::receipt_issued(
                env,
                client_receipt_id,
                job_id.clone(),
                0,
                escrow.client.clone(),
                client_award,
                0,
            );
        }

        if fee > 0 {
            if let Some(treasury_addr) = treasury {
                token_client.transfer(&env.current_contract_address(), &treasury_addr, &fee);
            }
        }

        if freelancer_payout > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.freelancer,
                &freelancer_payout,
            );
            let freelancer_receipt_id = next_receipt_id(env);
            EscrowEvents::receipt_issued(
                env,
                freelancer_receipt_id,
                job_id.clone(),
                0,
                escrow.freelancer.clone(),
                freelancer_payout,
                fee,
            );
        }

        EscrowEvents::dispute_resolved(env, job_id.clone(), client_award, freelancer_award);

        Ok(())
    }

    // ─── Query Methods ────────────────────────────────────────────────────────

    /// Get escrow metadata by job ID.
    pub fn get_escrow(env: Env, job_id: String) -> Result<Escrow, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(job_id))
            .ok_or(EscrowError::EscrowNotFound)
    }

    /// Get milestones for an escrow.
    pub fn get_milestones(env: Env, job_id: String) -> Result<Vec<Milestone>, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Milestones(job_id))
            .ok_or(EscrowError::EscrowNotFound)
    }

    /// Get registered deliverable hash.
    pub fn get_deliverable(env: Env, job_id: String) -> Result<BytesN<32>, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Deliverable(job_id))
            .ok_or(EscrowError::EscrowNotFound)
    }

    /// Get registered deliverable hash for a specific milestone ID.
    pub fn get_milestone_deliverable(
        env: Env,
        job_id: String,
        milestone_id: u32,
    ) -> Result<BytesN<32>, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::MilestoneDeliverable(job_id, milestone_id))
            .ok_or(EscrowError::MilestoneNotFound)
    }

    /// Get escrow status badge only.
    pub fn get_escrow_status(env: Env, job_id: String) -> Result<EscrowStatus, EscrowError> {
        let escrow = Self::get_escrow(env, job_id)?;
        Ok(escrow.status)
    }

    /// Get lightweight escrow financial progress summary.
    pub fn get_escrow_progress(env: Env, job_id: String) -> Result<EscrowProgress, EscrowError> {
        let escrow = Self::get_escrow(env, job_id)?;
        let pct = if escrow.total_amount > 0 {
            ((escrow.released_amount * 100) / escrow.total_amount) as u32
        } else {
            0
        };
        Ok(EscrowProgress {
            total_amount: escrow.total_amount,
            released_amount: escrow.released_amount,
            percentage_released: pct,
            status: escrow.status,
        })
    }

    /// Get a single milestone by ID.
    pub fn get_milestone(env: Env, job_id: String, milestone_id: u32) -> Result<Milestone, EscrowError> {
        let milestones = Self::get_milestones(env, job_id)?;
        for m in milestones.iter() {
            if m.id == milestone_id {
                return Ok(m);
            }
        }
        Err(EscrowError::MilestoneNotFound)
    }

    /// Get cast votes for a disputed escrow.
    pub fn get_dispute_votes(env: Env, job_id: String) -> Vec<ArbitratorVote> {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeBallot(job_id))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Query current bonded stake of an arbitrator.
    pub fn get_arbitrator_bond(env: Env, arbitrator: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ArbitratorBond(arbitrator))
            .unwrap_or(0)
    }

    /// Get dispute evidence hash submitted by a specific party.
    pub fn get_dispute_evidence(
        env: Env,
        job_id: String,
        caller: Address,
    ) -> Result<BytesN<32>, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::DisputeEvidence(job_id, caller))
            .ok_or(EscrowError::EscrowNotFound)
    }

    /// Sets a completion deadline timestamp for a specific milestone.
    /// Can only be called by the client while the contract is active.
    pub fn set_milestone_deadline(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
        deadline: u64,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        let escrow = Self::get_escrow(env.clone(), job_id.clone())?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestone = Self::get_milestone(env.clone(), job_id.clone(), milestone_id)?;
        if milestone.approved {
            return Err(EscrowError::MilestoneAlreadyApproved);
        }

        if deadline <= env.ledger().timestamp() {
            return Err(EscrowError::InvalidDeadline);
        }

        extend_storage_ttl(&env, &job_id);
        let key = DataKey::MilestoneDeadline(job_id.clone(), milestone_id);
        env.storage().persistent().set(&key, &deadline);
        env.storage().persistent().extend_ttl(&key, 100_000, 200_000);

        EscrowEvents::milestone_deadline_set(&env, job_id, milestone_id, deadline);
        Ok(())
    }

    /// Query the configured deadline for a milestone.
    pub fn get_milestone_deadline(
        env: Env,
        job_id: String,
        milestone_id: u32,
    ) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::MilestoneDeadline(job_id, milestone_id))
    }

    /// Check if a milestone deadline has expired without milestone approval.
    pub fn is_milestone_expired(
        env: Env,
        job_id: String,
        milestone_id: u32,
    ) -> bool {
        if let Some(deadline) = Self::get_milestone_deadline(env.clone(), job_id.clone(), milestone_id) {
            if let Ok(milestone) = Self::get_milestone(env.clone(), job_id, milestone_id) {
                if !milestone.approved && env.ledger().timestamp() >= deadline {
                    return true;
                }
            }
        }
        false
    }

    /// Cancels an unfulfilled milestone whose deadline has expired, refunding its allocated funds to the client.
    pub fn cancel_expired_milestone(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        let mut escrow = Self::get_escrow(env.clone(), job_id.clone())?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestone = Self::get_milestone(env.clone(), job_id.clone(), milestone_id)?;
        if milestone.approved {
            return Err(EscrowError::MilestoneAlreadyApproved);
        }

        if !Self::is_milestone_expired(env.clone(), job_id.clone(), milestone_id) {
            return Err(EscrowError::MilestoneNotExpired);
        }

        let mut milestones = Self::get_milestones(env.clone(), job_id.clone())?;
        let mut found = false;
        let mut refund_amount: i128 = 0;

        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.id == milestone_id {
                if m.approved {
                    return Err(EscrowError::MilestoneAlreadyApproved);
                }
                m.approved = true;
                refund_amount = m.amount;
                milestones.set(i, m);
                found = true;
                break;
            }
        }

        if !found {
            return Err(EscrowError::MilestoneNotFound);
        }

        extend_storage_ttl(&env, &job_id);

        // Refund milestone amount back to client without fee
        token::Client::new(&env, &escrow.token).transfer(
            &env.current_contract_address(),
            &client,
            &refund_amount,
        );
        let receipt_id = next_receipt_id(&env);
        EscrowEvents::receipt_issued(
            &env,
            receipt_id,
            job_id.clone(),
            milestone_id,
            client.clone(),
            refund_amount,
            0,
        );

        escrow.released_amount += refund_amount;
        if escrow.released_amount >= escrow.total_amount {
            escrow.status = EscrowStatus::Completed;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(job_id.clone()), &escrow);
        env.storage()
            .persistent()
            .set(&DataKey::Milestones(job_id.clone()), &milestones);

        EscrowEvents::milestone_cancelled(&env, job_id, milestone_id, refund_amount);
        Ok(())
    }

    /// Extends an unapproved milestone deadline by a specified number of seconds.
    pub fn extend_milestone_deadline(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
        extension_seconds: u64,
    ) -> Result<u64, EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        let escrow = Self::get_escrow(env.clone(), job_id.clone())?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestone = Self::get_milestone(env.clone(), job_id.clone(), milestone_id)?;
        if milestone.approved {
            return Err(EscrowError::MilestoneAlreadyApproved);
        }

        if extension_seconds == 0 || extension_seconds > 31_536_000 {
            return Err(EscrowError::InvalidDeadline);
        }

        let now = env.ledger().timestamp();
        let current_dl = Self::get_milestone_deadline(env.clone(), job_id.clone(), milestone_id).unwrap_or(now);
        let base = if current_dl > now { current_dl } else { now };
        let new_deadline = base + extension_seconds;

        extend_storage_ttl(&env, &job_id);
        let key = DataKey::MilestoneDeadline(job_id.clone(), milestone_id);
        env.storage().persistent().set(&key, &new_deadline);
        env.storage().persistent().extend_ttl(&key, 100_000, 200_000);

        EscrowEvents::milestone_deadline_extended(&env, job_id, milestone_id, new_deadline);
        Ok(new_deadline)
    }

    /// Releases a partial amount of a milestone to the freelancer with platform fee deducted pro-rata.
    /// If the partial release pays out the entire milestone amount, the milestone is marked approved.
    pub fn release_partial_milestone(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
        amount: i128,
    ) -> Result<i128, EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        let mut escrow = Self::get_escrow(env.clone(), job_id.clone())?;
        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestone = Self::get_milestone(env.clone(), job_id.clone(), milestone_id)?;
        if milestone.approved {
            return Err(EscrowError::MilestoneAlreadyApproved);
        }

        let released_key = DataKey::MilestoneReleased(job_id.clone(), milestone_id);
        let previously_released: i128 = env
            .storage()
            .persistent()
            .get(&released_key)
            .unwrap_or(0);

        let remaining = milestone.amount - previously_released;
        if amount <= 0 || amount > remaining {
            return Err(EscrowError::InvalidAmount);
        }

        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::FeeBps)
            .unwrap_or(0);
        let treasury: Address = env
            .storage()
            .instance()
            .get(&DataKey::Treasury)
            .ok_or(EscrowError::NotInitialized)?;

        let fee_amount = if fee_bps > 0 {
            amount
                .checked_mul(fee_bps as i128)
                .ok_or(EscrowError::InvalidAmount)?
                .checked_div(10_000)
                .ok_or(EscrowError::InvalidAmount)?
        } else {
            0
        };
        let freelancer_payout = amount
            .checked_sub(fee_amount)
            .ok_or(EscrowError::InvalidAmount)?;

        let token_client = token::Client::new(&env, &escrow.token);
        if fee_amount > 0 {
            token_client.transfer(&env.current_contract_address(), &treasury, &fee_amount);
        }
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &freelancer_payout,
        );

        let new_released = previously_released
            .checked_add(amount)
            .ok_or(EscrowError::InvalidAmount)?;
        env.storage().persistent().set(&released_key, &new_released);
        env.storage().persistent().extend_ttl(&released_key, 100_000, 200_000);

        let mut milestones = Self::get_milestones(env.clone(), job_id.clone())?;
        if new_released >= milestone.amount {
            for i in 0..milestones.len() {
                let mut m = milestones.get(i).unwrap();
                if m.id == milestone_id {
                    m.approved = true;
                    milestones.set(i, m);
                    break;
                }
            }
            env.storage()
                .persistent()
                .set(&DataKey::Milestones(job_id.clone()), &milestones);
        }

        escrow.released_amount += amount;
        if escrow.status == EscrowStatus::Created {
            escrow.status = EscrowStatus::InProgress;
        }
        if escrow.released_amount >= escrow.total_amount {
            escrow.status = EscrowStatus::Completed;
        }

        extend_storage_ttl(&env, &job_id);
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(job_id.clone()), &escrow);

        EscrowEvents::milestone_partial_released(
            &env,
            job_id.clone(),
            milestone_id,
            amount,
            freelancer_payout,
            fee_amount,
        );

        let receipt_id = next_receipt_id(&env);
        EscrowEvents::receipt_issued(
            &env,
            receipt_id,
            job_id,
            milestone_id,
            escrow.freelancer,
            amount,
            fee_amount,
        );

        Ok(milestone.amount - new_released)
    }

    /// Query the cumulative receipt counter across all releases.
    pub fn get_receipt_counter(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ReceiptCounter)
            .unwrap_or(0)
    }

    /// Query the cumulative amount released for a milestone.
    pub fn get_milestone_released_amount(
        env: Env,
        job_id: String,
        milestone_id: u32,
    ) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::MilestoneReleased(job_id, milestone_id))
            .unwrap_or(0)
    }

    /// Query the remaining unreleased amount for a milestone.
    pub fn get_milestone_remaining_amount(
        env: Env,
        job_id: String,
        milestone_id: u32,
    ) -> Result<i128, EscrowError> {
        let milestone = Self::get_milestone(env.clone(), job_id.clone(), milestone_id)?;
        if milestone.approved {
            return Ok(0);
        }
        let released = Self::get_milestone_released_amount(env, job_id, milestone_id);
        Ok(milestone.amount - released)
    }

    /// Archive a fully finalized escrow (status must be Completed or Refunded, with 0 residual balance).
    pub fn archive_completed_escrow(env: Env, job_id: String) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.status != EscrowStatus::Completed && escrow.status != EscrowStatus::Refunded {
            return Err(EscrowError::InvalidStatus);
        }

        if escrow.released_amount != escrow.total_amount {
            return Err(EscrowError::InvalidAmount);
        }

        let milestones_key = DataKey::Milestones(job_id.clone());
        env.storage().persistent().remove(&milestones_key);

        let votes_key = DataKey::DisputeBallot(job_id.clone());
        env.storage().persistent().remove(&votes_key);

        EscrowEvents::escrow_archived(&env, job_id);

        Ok(())
    }

    /// Set minimum cooling-off period required before an arbitrator can resolve a dispute (admin only).
    pub fn set_dispute_cooldown(
        env: Env,
        admin: Address,
        cooldown_seconds: u64,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::DisputeCooldown, &cooldown_seconds);

        Ok(())
    }

    /// Query the configured dispute cooling-off period in seconds.
    pub fn get_dispute_cooldown(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeCooldown)
            .unwrap_or(0)
    }

    /// Record an on-chain cryptographic audit entry with strictly monotonic counter
    pub fn record_audit_entry(
        env: Env,
        caller: Address,
        action: Symbol,
        record_hash: BytesN<32>,
    ) -> Result<u64, EscrowError> {
        ensure_not_paused(&env)?;
        caller.require_auth();

        let current_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AuditLogCounter)
            .unwrap_or(0);
        let next_count = current_count + 1;
        env.storage()
            .instance()
            .set(&DataKey::AuditLogCounter, &next_count);
        env.storage()
            .persistent()
            .set(&DataKey::AuditLogReceipt(next_count), &record_hash);

        EscrowEvents::audit_log_recorded(&env, next_count, action, record_hash);
        Ok(next_count)
    }

    /// Query the total number of recorded on-chain audit logs.
    pub fn get_audit_log_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AuditLogCounter)
            .unwrap_or(0)
    }

    /// Verify an audit receipt hash against the immutable on-chain record.
    pub fn verify_audit_receipt(
        env: Env,
        index: u64,
        expected_hash: BytesN<32>,
    ) -> Result<bool, EscrowError> {
        let stored_hash: Option<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::AuditLogReceipt(index));
        match stored_hash {
            Some(h) => Ok(h == expected_hash),
            None => Err(EscrowError::AuditIndexMismatch),
        }
    }

    /// Set upgrade timelock delay in seconds (admin only, minimum 3600 seconds).
    pub fn set_upgrade_delay(
        env: Env,
        admin: Address,
        delay_seconds: u64,
    ) -> Result<(), EscrowError> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        if delay_seconds < 3600 {
            return Err(EscrowError::InvalidUpgradeDelay);
        }

        env.storage()
            .instance()
            .set(&DataKey::UpgradeDelay, &delay_seconds);
        Ok(())
    }

    /// Query the upgrade timelock delay in seconds (defaults to 86_400).
    pub fn get_upgrade_delay(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::UpgradeDelay)
            .unwrap_or(86_400)
    }

    /// Stage a contract Wasm bytecode upgrade with timelock protection (admin only).
    pub fn stage_upgrade(
        env: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<u64, EscrowError> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        let delay = Self::get_upgrade_delay(env.clone());
        let unlock_timestamp = env.ledger().timestamp() + delay;

        env.storage()
            .instance()
            .set(&DataKey::StagedUpgradeHash, &new_wasm_hash);
        env.storage()
            .instance()
            .set(&DataKey::StagedUpgradeUnlock, &unlock_timestamp);

        EscrowEvents::upgrade_staged(&env, admin, new_wasm_hash, unlock_timestamp);
        Ok(unlock_timestamp)
    }

    /// Query the currently staged contract upgrade, if any.
    pub fn get_staged_upgrade(env: Env) -> Option<(BytesN<32>, u64)> {
        let hash: BytesN<32> = env.storage().instance().get(&DataKey::StagedUpgradeHash)?;
        let unlock_ts: u64 = env.storage().instance().get(&DataKey::StagedUpgradeUnlock)?;
        Some((hash, unlock_ts))
    }

    /// Cancel a previously staged contract upgrade (admin only).
    pub fn cancel_staged_upgrade(env: Env, admin: Address) -> Result<(), EscrowError> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        if let Some(hash) = env.storage().instance().get::<_, BytesN<32>>(&DataKey::StagedUpgradeHash) {
            EscrowEvents::upgrade_cancelled(&env, admin, hash);
        }

        env.storage().instance().remove(&DataKey::StagedUpgradeHash);
        env.storage().instance().remove(&DataKey::StagedUpgradeUnlock);
        Ok(())
    }

    /// Execute the staged contract upgrade once timelock delay has elapsed (admin only).
    pub fn execute_upgrade(env: Env, admin: Address) -> Result<(), EscrowError> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        let staged_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::StagedUpgradeHash)
            .ok_or(EscrowError::UpgradeNotStaged)?;

        let unlock_timestamp: u64 = env
            .storage()
            .instance()
            .get(&DataKey::StagedUpgradeUnlock)
            .ok_or(EscrowError::UpgradeNotStaged)?;

        let now = env.ledger().timestamp();
        if now < unlock_timestamp {
            return Err(EscrowError::UpgradeTimelockActive);
        }

        env.deployer().update_current_contract_wasm(staged_hash.clone());

        env.storage().instance().remove(&DataKey::StagedUpgradeHash);
        env.storage().instance().remove(&DataKey::StagedUpgradeUnlock);

        EscrowEvents::upgrade_executed(&env, admin, staged_hash);
        Ok(())
    }

    /// Create a governance proposal with a voting duration and quorum threshold.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        title: String,
        description: String,
        quorum_threshold: i128,
        duration_seconds: u64,
    ) -> Result<u64, EscrowError> {
        ensure_not_paused(&env)?;
        proposer.require_auth();

        if quorum_threshold <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let current_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        let proposal_id = current_count + 1;
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &proposal_id);

        let end_timestamp = env.ledger().timestamp() + duration_seconds;
        let proposal = GovernanceProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            title,
            description,
            votes_for: 0,
            votes_against: 0,
            quorum_threshold,
            end_timestamp,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        EscrowEvents::proposal_created(&env, proposal_id, proposer);
        Ok(proposal_id)
    }

    /// Cast a weighted vote on an active governance proposal.
    pub fn vote_proposal(
        env: Env,
        proposal_id: u64,
        voter: Address,
        support: bool,
        weight: i128,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        voter.require_auth();

        if weight <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let vote_key = DataKey::ProposalVote(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(EscrowError::ProposalAlreadyVoted);
        }

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: GovernanceProposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .ok_or(EscrowError::ProposalNotFound)?;

        if env.ledger().timestamp() > proposal.end_timestamp {
            return Err(EscrowError::ProposalExpired);
        }

        if support {
            proposal.votes_for += weight;
        } else {
            proposal.votes_against += weight;
        }

        env.storage().persistent().set(&proposal_key, &proposal);
        env.storage().persistent().set(&vote_key, &true);

        EscrowEvents::proposal_voted(&env, proposal_id, voter, support, weight);
        Ok(())
    }

    /// Query a governance proposal by ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<GovernanceProposal, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(EscrowError::ProposalNotFound)
    }

    /// Determine whether a governance proposal met quorum and passed.
    pub fn is_proposal_passed(env: Env, proposal_id: u64) -> Result<bool, EscrowError> {
        let proposal = Self::get_proposal(env, proposal_id)?;
        let total_votes = proposal.votes_for + proposal.votes_against;
        let quorum_met = total_votes >= proposal.quorum_threshold;
        let passed = quorum_met && (proposal.votes_for > proposal.votes_against);
        Ok(passed)
    }

    /// Submit deliverable hash verifying that the caller is an authorized party of the escrow (freelancer or client).
    pub fn submit_deliverable_by_party(
        env: Env,
        job_id: String,
        caller: Address,
        deliverable_hash: BytesN<32>,
    ) -> Result<(), EscrowError> {
        ensure_not_paused(&env)?;
        caller.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if caller != escrow.freelancer && caller != escrow.client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Deliverable(job_id.clone()), &deliverable_hash);

        extend_storage_ttl(&env, &job_id);
        EscrowEvents::deliverable_submitted(&env, job_id, deliverable_hash);
        Ok(())
    }

    /// Query the current replay protection nonce for an account.
    pub fn get_account_nonce(env: Env, account: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AccountNonce(account))
            .unwrap_or(0)
    }

    /// Verifies and increments the anti-replay nonce for an account (must be strictly expected_nonce = current + 1).
    pub fn verify_and_increment_nonce(
        env: Env,
        account: Address,
        expected_nonce: u64,
    ) -> Result<u64, EscrowError> {
        account.require_auth();
        let current = Self::get_account_nonce(env.clone(), account.clone());
        if expected_nonce != current + 1 {
            return Err(EscrowError::InvalidNonce);
        }
        env.storage()
            .instance()
            .set(&DataKey::AccountNonce(account), &expected_nonce);
        Ok(expected_nonce)
    }

    /// Gas-optimized batch release: approves multiple milestones in a single transaction with consolidated token settlement.
    pub fn batch_release_milestones(
        env: Env,
        job_id: String,
        client: Address,
        milestone_ids: Vec<u32>,
    ) -> Result<i128, EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        if milestone_ids.is_empty() || milestone_ids.len() > MAX_MILESTONES {
            return Err(EscrowError::InvalidMilestoneCount);
        }

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&escrow_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        if escrow.client != client {
            return Err(EscrowError::Unauthorized);
        }

        if escrow.status != EscrowStatus::InProgress && escrow.status != EscrowStatus::Created {
            return Err(EscrowError::InvalidStatus);
        }

        let milestones_key = DataKey::Milestones(job_id.clone());
        let milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&milestones_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        let mut total_release_amount: i128 = 0;
        let mut updated_milestones: Vec<Milestone> = Vec::new(&env);
        let mut all_completed = true;

        for mut m in milestones.iter() {
            let mut should_approve = false;
            for target_id in milestone_ids.iter() {
                if m.id == target_id {
                    should_approve = true;
                    break;
                }
            }

            if should_approve {
                if m.approved {
                    return Err(EscrowError::MilestoneAlreadyApproved);
                }
                m.approved = true;
                total_release_amount = total_release_amount
                    .checked_add(m.amount)
                    .ok_or(EscrowError::InvalidAmount)?;
                EscrowEvents::milestone_released(&env, job_id.clone(), m.id, m.amount);
            }

            if !m.approved {
                all_completed = false;
            }
            updated_milestones.push_back(m);
        }

        if total_release_amount == 0 {
            return Err(EscrowError::MilestoneNotFound);
        }

        // Checks & Effects: CEI pattern
        escrow.released_amount += total_release_amount;
        if all_completed || escrow.released_amount >= escrow.total_amount {
            escrow.status = EscrowStatus::Completed;
        } else {
            escrow.status = EscrowStatus::InProgress;
        }

        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage()
            .persistent()
            .set(&milestones_key, &updated_milestones);
        extend_storage_ttl(&env, &job_id);

        // Interactions: Single consolidated token settlement
        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let treasury: Option<Address> = env.storage().instance().get(&DataKey::Treasury);

        let fee = total_release_amount
            .checked_mul(fee_bps as i128)
            .ok_or(EscrowError::InvalidAmount)?
            .checked_div(10_000)
            .ok_or(EscrowError::InvalidAmount)?;
        let freelancer_payout = total_release_amount
            .checked_sub(fee)
            .ok_or(EscrowError::InvalidAmount)?;

        let token_client = token::Client::new(&env, &escrow.token);

        if fee > 0 {
            if let Some(treasury_addr) = treasury {
                token_client.transfer(&env.current_contract_address(), &treasury_addr, &fee);
            }
        }

        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &freelancer_payout,
        );

        Ok(total_release_amount)
    }

    /// Read-only preview method calculating consolidated settlement totals and fees before release.
    pub fn preview_batch_release(
        env: Env,
        job_id: String,
        milestone_ids: Vec<u32>,
    ) -> Result<(i128, i128, i128), EscrowError> {
        if milestone_ids.is_empty() || milestone_ids.len() > MAX_MILESTONES {
            return Err(EscrowError::InvalidMilestoneCount);
        }

        let milestones_key = DataKey::Milestones(job_id);
        let milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&milestones_key)
            .ok_or(EscrowError::EscrowNotFound)?;

        let mut total_release_amount: i128 = 0;
        for m in milestones.iter() {
            for target_id in milestone_ids.iter() {
                if m.id == target_id {
                    if m.approved {
                        return Err(EscrowError::MilestoneAlreadyApproved);
                    }
                    total_release_amount = total_release_amount
                        .checked_add(m.amount)
                        .ok_or(EscrowError::InvalidAmount)?;
                    break;
                }
            }
        }

        if total_release_amount == 0 {
            return Err(EscrowError::MilestoneNotFound);
        }

        let fee_bps: u32 = env.storage().instance().get(&DataKey::FeeBps).unwrap_or(0);
        let fee = total_release_amount
            .checked_mul(fee_bps as i128)
            .ok_or(EscrowError::InvalidAmount)?
            .checked_div(10_000)
            .ok_or(EscrowError::InvalidAmount)?;
        let freelancer_payout = total_release_amount
            .checked_sub(fee)
            .ok_or(EscrowError::InvalidAmount)?;

        Ok((total_release_amount, fee, freelancer_payout))
    }

    /// Gas-optimized batch cancellation: cancels multiple inactive escrows created by the client and refunds deposits.
    pub fn batch_cancel_escrows(
        env: Env,
        client: Address,
        job_ids: Vec<String>,
    ) -> Result<i128, EscrowError> {
        ensure_not_paused(&env)?;
        client.require_auth();

        if job_ids.is_empty() || job_ids.len() > 10 {
            return Err(EscrowError::InvalidMilestoneCount);
        }

        let mut total_refund: i128 = 0;

        for job_id in job_ids.iter() {
            let escrow_key = DataKey::Escrow(job_id.clone());
            let mut escrow: Escrow = env
                .storage()
                .persistent()
                .get(&escrow_key)
                .ok_or(EscrowError::EscrowNotFound)?;

            if escrow.client != client {
                return Err(EscrowError::Unauthorized);
            }

            if escrow.status != EscrowStatus::Created {
                return Err(EscrowError::InvalidStatus);
            }

            let refund_amount = escrow
                .total_amount
                .checked_sub(escrow.released_amount)
                .ok_or(EscrowError::InvalidAmount)?;

            escrow.status = EscrowStatus::Refunded;
            env.storage().persistent().set(&escrow_key, &escrow);
            extend_storage_ttl(&env, &job_id);

            EscrowEvents::escrow_refunded(&env, job_id, refund_amount);

            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(
                &env.current_contract_address(),
                &client,
                &refund_amount,
            );

            total_refund = total_refund
                .checked_add(refund_amount)
                .ok_or(EscrowError::InvalidAmount)?;
        }

        Ok(total_refund)
    }
}
