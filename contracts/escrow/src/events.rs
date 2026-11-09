use soroban_sdk::{symbol_short, Address, BytesN, Env, String};

pub struct EscrowEvents;

impl EscrowEvents {
    pub fn pause(env: &Env, admin: Address) {
        env.events().publish((symbol_short!("pause"),), admin);
    }

    pub fn unpause(env: &Env, admin: Address) {
        env.events().publish((symbol_short!("unpause"),), admin);
    }

    pub fn set_treasury(env: &Env, new_treasury: Address) {
        env.events()
            .publish((symbol_short!("admin"), symbol_short!("treasury")), new_treasury);
    }

    pub fn transfer_admin(env: &Env, new_admin: Address) {
        env.events()
            .publish((symbol_short!("admin"), symbol_short!("transfer")), new_admin);
    }

    pub fn escrow_created(env: &Env, job_id: String, total_amount: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("created")), (job_id, total_amount));
    }

    pub fn escrow_started(env: &Env, job_id: String) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("started")), job_id);
    }

    pub fn deliverable_submitted(env: &Env, job_id: String, deliverable_hash: BytesN<32>) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("deliv")), (job_id, deliverable_hash));
    }

    pub fn milestone_deliverable_submitted(env: &Env, job_id: String, milestone_id: u32, deliverable_hash: BytesN<32>) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("ms_deliv")), (job_id, milestone_id, deliverable_hash));
    }

    pub fn milestone_released(env: &Env, job_id: String, milestone_id: u32, release_amount: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("released")), (job_id, milestone_id, release_amount));
    }

    pub fn escrow_refunded(env: &Env, job_id: String, refunded_amount: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("refunded")), (job_id, refunded_amount));
    }

    pub fn mutual_cancelled(env: &Env, job_id: String, refunded_amount: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("mut_canc")), (job_id, refunded_amount));
    }

    pub fn timeout_refunded(env: &Env, job_id: String, refunded_amount: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("timeout")), (job_id, refunded_amount));
    }

    pub fn dispute_raised(env: &Env, job_id: String, caller: Address) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("disputed")), (job_id, caller));
    }

    pub fn dispute_resolved(env: &Env, job_id: String, client_award: i128, freelancer_award: i128) {
        env.events()
            .publish((symbol_short!("escrow"), symbol_short!("resolved")), (job_id, client_award, freelancer_award));
    }

    pub fn arbitrator_registered(env: &Env, arbitrator: Address, bond_amount: i128) {
        env.events()
            .publish((symbol_short!("arb"), symbol_short!("bonded")), (arbitrator, bond_amount));
    }
}
