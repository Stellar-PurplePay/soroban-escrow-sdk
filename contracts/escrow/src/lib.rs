#![no_std]
#![allow(deprecated)]

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, String, Vec};

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

    /// Update platform treasury recipient address (admin only).
    pub fn set_treasury(env: Env, new_treasury: Address) -> Result<(), EscrowError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

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
        if params.timeout_timestamp <= now {
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

        let fee = (release_amount * (fee_bps as i128)) / 10_000;
        let freelancer_payout = release_amount - fee;

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

        EscrowEvents::milestone_released(&env, job_id, milestone_id, release_amount);

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

        let fee = (freelancer_award * (fee_bps as i128)) / 10_000;
        let freelancer_payout = freelancer_award - fee;

        let token_client = token::Client::new(env, &escrow.token);

        if client_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.client,
                &client_award,
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
}
