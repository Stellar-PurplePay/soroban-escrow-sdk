#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, BytesN, Env, String, Vec};

fn setup_test<'a>(
    env: &'a Env,
) -> (
    Address,
    Address,
    Address,
    Address,
    Address,
    token::Client<'a>,
    token::StellarAssetClient<'a>,
    Address,
    EscrowContractClient<'a>,
) {
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    let client = Address::generate(env);
    let freelancer = Address::generate(env);
    let arbitrator = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::Client::new(env, &token_contract_id.address());
    let token_admin_client = token::StellarAssetClient::new(env, &token_contract_id.address());

    // Mint tokens to client
    token_admin_client.mint(&client, &10_000_000);

    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(env, &escrow_contract_id);

    // Initialize with 100 bps (1%) platform fee
    escrow_client.initialize(&admin, &treasury, &100);

    (
        admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        token_admin_client,
        token_contract_id.address(),
        escrow_client,
    )
}

#[test]
fn test_escrow_creation_and_full_release() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin_client,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-101");
    let timeout = env.ledger().timestamp() + 10_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let milestones = Vec::new(&env);
    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // Check escrow record
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.total_amount, 1_000_000);
    assert_eq!(escrow.status, EscrowStatus::Created);

    // Freelancer starts work
    escrow_client.start_work(&job_id, &freelancer);
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);

    // Submit deliverable hash
    let dummy_hash = BytesN::from_array(&env, &[7u8; 32]);
    escrow_client.submit_deliverable(&job_id, &freelancer, &dummy_hash);
    assert_eq!(escrow_client.get_deliverable(&job_id), dummy_hash);

    // Client releases milestone (milestone 1 is auto-created 100%)
    escrow_client.release_milestone(&job_id, &client, &1);

    // Verify payout: 1% fee = 10,000 to treasury, 990,000 to freelancer
    assert_eq!(token_client.balance(&treasury), 10_000);
    assert_eq!(token_client.balance(&freelancer), 990_000);

    let updated_escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(updated_escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_milestone_pro_rata_releases() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin_client,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-milestones-1");
    let timeout = env.ledger().timestamp() + 20_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 2_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Phase 1: Architecture"),
        percentage: 40,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Phase 2: Final Delivery"),
        percentage: 60,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // Release Milestone 1 (40% = 800,000)
    // Fee = 1% of 800,000 = 8,000, Freelancer receives 792,000
    escrow_client.release_milestone(&job_id, &client, &1);
    assert_eq!(token_client.balance(&treasury), 8_000);
    assert_eq!(token_client.balance(&freelancer), 792_000);

    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::InProgress);

    // Release Milestone 2 (60% = 1,200,000)
    // Fee = 12,000 (total treasury = 20,000), Freelancer receives 1,188,000 (total = 1,980,000)
    escrow_client.release_milestone(&job_id, &client, &2);
    assert_eq!(token_client.balance(&treasury), 20_000);
    assert_eq!(token_client.balance(&freelancer), 1_980_000);

    let final_escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(final_escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_cancel_and_refund_before_work_starts() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin_client,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-cancel-test");
    let timeout = env.ledger().timestamp() + 5_000;

    let initial_balance = token_client.balance(&client);

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 500_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let milestones = Vec::new(&env);
    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    assert_eq!(token_client.balance(&client), initial_balance - 500_000);

    // Cancel and refund
    escrow_client.cancel_and_refund(&job_id, &client);
    assert_eq!(token_client.balance(&client), initial_balance);

    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn test_permissionless_timeout_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin_client,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-timeout-test");
    let timeout = env.ledger().timestamp() + 500;

    let initial_balance = token_client.balance(&client);

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 750_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let milestones = Vec::new(&env);
    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // Advance timestamp past timeout
    env.ledger().set_timestamp(timeout + 10);

    // Anyone can claim timeout refund
    escrow_client.claim_timeout_refund(&job_id);

    assert_eq!(token_client.balance(&client), initial_balance);
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn test_dispute_and_arbitration_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin_client,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-dispute-test");
    let timeout = env.ledger().timestamp() + 10_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let milestones = Vec::new(&env);
    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    // Client raises dispute
    escrow_client.raise_dispute(&job_id, &client);
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);

    let client_prev_balance = token_client.balance(&client);
    let freelancer_prev_balance = token_client.balance(&freelancer);

    // Arbitrator resolves: 600,000 back to client, 400,000 to freelancer
    escrow_client.resolve_dispute(&job_id, &arbitrator, &600_000, &400_000);

    assert_eq!(token_client.balance(&client), client_prev_balance + 600_000);
    assert_eq!(
        token_client.balance(&freelancer),
        freelancer_prev_balance + 400_000
    );

    let resolved_escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(resolved_escrow.status, EscrowStatus::Completed);
}

