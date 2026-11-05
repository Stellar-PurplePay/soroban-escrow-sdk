#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contractimpl, symbol_short, token, Address, BytesN, Env, String, Vec,
};

mod errors;
mod types;

#[cfg(test)]
mod test;

pub use errors::EscrowError;
pub use types::*;

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

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);

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

        env.storage().instance().set(&DataKey::FeeBps, &fee_bps);
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
        client.require_auth();

        if params.total_amount <= 0 {
            return Err(EscrowError::InvalidAmount);
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
                total_pct += m.percentage;
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
        let token_client = token::Client::new(&env, &params.token);
        token_client.transfer(&client, &env.current_contract_address(), &params.total_amount);

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

        env.storage().persistent().extend_ttl(&escrow_key, 100_000, 200_000);
        env.storage().persistent().extend_ttl(&DataKey::Milestones(job_id.clone()), 100_000, 200_000);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("created")),
            (job_id, escrow.total_amount),
        );

        Ok(())
    }

    /// Freelancer indicates work has commenced, transitioning status from Created to InProgress.
    pub fn start_work(env: Env, job_id: String, freelancer: Address) -> Result<(), EscrowError> {
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

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("started")),
            job_id,
        );

        Ok(())
    }

    /// Freelancer registers a cryptographic proof hash (e.g. SHA-256 or IPFS CID) for a deliverable.
    pub fn submit_deliverable(
        env: Env,
        job_id: String,
        freelancer: Address,
        deliverable_hash: BytesN<32>,
    ) -> Result<(), EscrowError> {
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

        env.storage().persistent().set(
            &DataKey::Deliverable(job_id.clone()),
            &deliverable_hash,
        );

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("deliv")),
            (job_id, deliverable_hash),
        );

        Ok(())
    }

    /// Client approves a milestone, releasing pro-rata funds to the freelancer less treasury fee.
    pub fn release_milestone(
        env: Env,
        job_id: String,
        client: Address,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
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

        // Payout settlement
        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::FeeBps)
            .unwrap_or(0);
        let treasury: Option<Address> = env.storage().instance().get(&DataKey::Treasury);

        let fee = (release_amount * (fee_bps as i128)) / 10_000;
        let freelancer_payout = release_amount - fee;

        let token_client = token::Client::new(&env, &escrow.token);

        if fee > 0 {
            if let Some(treasury_addr) = treasury {
                token_client.transfer(
                    &env.current_contract_address(),
                    &treasury_addr,
                    &fee,
                );
            }
        }

        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &freelancer_payout,
        );

        escrow.released_amount += release_amount;
        if all_completed || escrow.released_amount >= escrow.total_amount {
            escrow.status = EscrowStatus::Completed;
        } else {
            escrow.status = EscrowStatus::InProgress;
        }

        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage().persistent().set(&milestones_key, &updated_milestones);
        env.storage().persistent().extend_ttl(&escrow_key, 100_000, 200_000);
        env.storage().persistent().extend_ttl(&milestones_key, 100_000, 200_000);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("released")),
            (job_id, milestone_id, release_amount),
        );

        Ok(())
    }

    /// Client cancels the escrow and reclaims unreleased funds if work has not yet commenced.
    pub fn cancel_and_refund(
        env: Env,
        job_id: String,
        client: Address,
    ) -> Result<(), EscrowError> {
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
        if remaining > 0 {
            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(&env.current_contract_address(), &escrow.client, &remaining);
        }

        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&escrow_key, &escrow);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("refunded")),
            (job_id, remaining),
        );

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
        let is_standard_timeout = (escrow.status == EscrowStatus::Created || escrow.status == EscrowStatus::InProgress)
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
        if remaining > 0 {
            let token_client = token::Client::new(&env, &escrow.token);
            token_client.transfer(&env.current_contract_address(), &escrow.client, &remaining);
        }

        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&escrow_key, &escrow);
        env.storage().persistent().extend_ttl(&escrow_key, 100_000, 200_000);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("timeout")),
            (job_id, remaining),
        );

        Ok(())
    }

    /// Raise a dispute on an active escrow (invoked by client or freelancer).
    pub fn raise_dispute(env: Env, job_id: String, caller: Address) -> Result<(), EscrowError> {
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

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("disputed")),
            (job_id, caller),
        );

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
        arbitrator.require_auth();

        let escrow_key = DataKey::Escrow(job_id.clone());
        let mut escrow: Escrow = env
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
        if client_award < 0 || freelancer_award < 0 || (client_award + freelancer_award) != remaining {
            return Err(EscrowError::InvalidAmount);
        }

        let token_client = token::Client::new(&env, &escrow.token);

        if client_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.client,
                &client_award,
            );
        }

        if freelancer_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.freelancer,
                &freelancer_award,
            );
        }

        escrow.released_amount += freelancer_award;
        escrow.status = EscrowStatus::Completed;
        env.storage().persistent().set(&escrow_key, &escrow);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("resolved")),
            (job_id, client_award, freelancer_award),
        );

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
}

