#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String, Vec,
};

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
        treasury,
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
    let treasury_prev_balance = token_client.balance(&treasury);

    // Arbitrator resolves: 600,000 back to client, 400,000 to freelancer (with 1% fee = 4,000 to treasury)
    escrow_client.resolve_dispute(&job_id, &arbitrator, &600_000, &400_000);

    assert_eq!(token_client.balance(&client), client_prev_balance + 600_000);
    assert_eq!(
        token_client.balance(&freelancer),
        freelancer_prev_balance + 396_000
    );
    assert_eq!(
        token_client.balance(&treasury),
        treasury_prev_balance + 4_000
    );

    let resolved_escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(resolved_escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_fee_ceiling_enforced() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    // Fee above 1,000 bps (10.00%) must fail at initialization
    let res = escrow_client.try_initialize(&admin, &treasury, &1_001);
    assert_eq!(res, Err(Ok(EscrowError::InvalidFee)));

    // Fee of u32::MAX must fail
    let res_max = escrow_client.try_initialize(&admin, &treasury, &u32::MAX);
    assert_eq!(res_max, Err(Ok(EscrowError::InvalidFee)));

    // Fee within ceiling succeeds
    escrow_client.initialize(&admin, &treasury, &1_000);

    // Dynamic fee update above ceiling must fail
    let res_update = escrow_client.try_set_fee_bps(&1_001);
    assert_eq!(res_update, Err(Ok(EscrowError::InvalidFee)));

    // Zero fee update is valid
    assert!(escrow_client.try_set_fee_bps(&0).is_ok());
}

#[test]
fn test_milestone_validation_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-milestone-ceiling");
    let timeout = env.ledger().timestamp() + 3600;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    // 1. Exceeding 25 milestones must be rejected
    let mut excess_milestones = Vec::new(&env);
    for _ in 0..26 {
        excess_milestones.push_back(MilestoneInput {
            description: String::from_str(&env, "task"),
            percentage: 3,
        });
    }
    let res_count = escrow_client.try_create_escrow(&job_id, &client, &params, &excess_milestones);
    assert_eq!(res_count, Err(Ok(EscrowError::InvalidMilestoneCount)));

    // 2. 0% milestone must be rejected
    let mut zero_milestones = Vec::new(&env);
    zero_milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "zero"),
        percentage: 0,
    });
    zero_milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "hundred"),
        percentage: 100,
    });
    let res_zero = escrow_client.try_create_escrow(&job_id, &client, &params, &zero_milestones);
    assert_eq!(res_zero, Err(Ok(EscrowError::InvalidMilestonePercentages)));

    // 3. Empty description must be rejected
    let mut empty_desc_milestones = Vec::new(&env);
    empty_desc_milestones.push_back(MilestoneInput {
        description: String::from_str(&env, ""),
        percentage: 100,
    });
    let res_empty =
        escrow_client.try_create_escrow(&job_id, &client, &params, &empty_desc_milestones);
    assert_eq!(res_empty, Err(Ok(EscrowError::InvalidMilestoneData)));
}

#[test]
fn test_emergency_pause_and_unpause_circuit_breaker() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    assert_eq!(escrow_client.is_paused(), false);

    // 1. Pause contract
    escrow_client.pause();
    assert_eq!(escrow_client.is_paused(), true);

    let job_id = String::from_str(&env, "job-paused");
    let timeout = env.ledger().timestamp() + 3600;
    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 100_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    // 2. create_escrow must fail when paused
    let res = escrow_client.try_create_escrow(&job_id, &client, &params, &Vec::new(&env));
    assert_eq!(res, Err(Ok(EscrowError::ContractPaused)));

    // 3. Unpause contract
    escrow_client.unpause();
    assert_eq!(escrow_client.is_paused(), false);

    // 4. create_escrow succeeds once unpaused
    let res_after = escrow_client.try_create_escrow(&job_id, &client, &params, &Vec::new(&env));
    assert!(res_after.is_ok());
}

#[test]
fn test_mutual_cancellation_while_in_progress() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-mutual-cancel");
    let timeout = env.ledger().timestamp() + 3600;
    let initial_balance = 1_000_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: initial_balance,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    escrow_client.create_escrow(&job_id, &client, &params, &Vec::new(&env));
    escrow_client.start_work(&job_id, &freelancer);

    // Mutual cancel by client and freelancer
    escrow_client.mutual_cancel(&job_id);

    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Refunded);

    // Client received remaining funds back
    assert_eq!(token_client.balance(&client), 10_000_000);
}

#[test]
fn test_lightweight_query_methods() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-queries");
    let timeout = env.ledger().timestamp() + 3600;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "M1"),
        percentage: 40,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "M2"),
        percentage: 60,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // 1. Check status
    assert_eq!(escrow_client.get_escrow_status(&job_id), EscrowStatus::Created);

    // 2. Check single milestone lookup
    let m1 = escrow_client.get_milestone(&job_id, &1);
    assert_eq!(m1.percentage, 40);
    assert_eq!(m1.amount, 400_000);

    // 3. Release M1 and verify progress
    escrow_client.start_work(&job_id, &freelancer);
    escrow_client.release_milestone(&job_id, &client, &1);

    let progress = escrow_client.get_escrow_progress(&job_id);
    assert_eq!(progress.released_amount, 400_000);
    assert_eq!(progress.total_amount, 1_000_000);
    assert_eq!(progress.percentage_released, 40);
    assert_eq!(progress.status, EscrowStatus::InProgress);
}

#[test]
fn test_treasury_rotation_and_admin_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        _client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    let new_treasury = Address::generate(&env);
    let new_admin = Address::generate(&env);

    // 1. Rotate treasury
    escrow_client.set_treasury(&new_treasury);

    // 2. Transfer admin
    escrow_client.set_admin(&new_admin);

    // 3. New admin can configure fee
    escrow_client.set_fee_bps(&250);
}

#[test]
fn test_property_milestone_sum_conservation() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    // Test a matrix of various milestone percentage combinations summing to 100%
    let test_splits: [&[u32]; 7] = [
        &[100],
        &[50, 50],
        &[33, 33, 34],
        &[25, 25, 25, 25],
        &[10, 20, 30, 25, 15],
        &[14, 14, 14, 14, 14, 14, 16],
        &[10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
    ];

    let total_deposit: i128 = 10_000_000; // 10 million units

    for (test_idx, split) in test_splits.iter().enumerate() {
        // Mint additional tokens to client for each test case
        token_admin.mint(&client, &total_deposit);

        let initial_freelancer_balance = token_client.balance(&freelancer);
        let initial_treasury_balance = token_client.balance(&treasury);

        let job_id = match test_idx {
            0 => String::from_str(&env, "job-prop-split-0"),
            1 => String::from_str(&env, "job-prop-split-1"),
            2 => String::from_str(&env, "job-prop-split-2"),
            3 => String::from_str(&env, "job-prop-split-3"),
            4 => String::from_str(&env, "job-prop-split-4"),
            5 => String::from_str(&env, "job-prop-split-5"),
            _ => String::from_str(&env, "job-prop-split-6"),
        };
        let timeout = env.ledger().timestamp() + 50_000;

        let params = CreateEscrowParams {
            freelancer: freelancer.clone(),
            token: token_address.clone(),
            total_amount: total_deposit,
            timeout_timestamp: timeout,
            arbitrator: arbitrator.clone(),
        };

        let mut milestones = Vec::new(&env);
        for (_m_idx, &pct) in split.iter().enumerate() {
            milestones.push_back(MilestoneInput {
                description: String::from_str(&env, "Phase"),
                percentage: pct,
            });
        }

        escrow_client.create_escrow(&job_id, &client, &params, &milestones);
        escrow_client.start_work(&job_id, &freelancer);

        // Release all milestones
        for m_id in 1..=(split.len() as u32) {
            escrow_client.release_milestone(&job_id, &client, &m_id);
        }

        // Verify Conservation of Funds Invariant:
        // net freelancer payout + net treasury fees == total_deposit
        let final_freelancer_balance = token_client.balance(&freelancer);
        let final_treasury_balance = token_client.balance(&treasury);

        let total_paid_to_freelancer = final_freelancer_balance - initial_freelancer_balance;
        let total_paid_to_treasury = final_treasury_balance - initial_treasury_balance;
        let total_disbursed = total_paid_to_freelancer + total_paid_to_treasury;

        assert_eq!(
            total_disbursed, total_deposit,
            "Invariant violation at split test index {}: disbursed ({}) != deposit ({})",
            test_idx, total_disbursed, total_deposit
        );

        let escrow = escrow_client.get_escrow(&job_id);
        assert_eq!(escrow.status, EscrowStatus::Completed);
        assert_eq!(escrow.released_amount, total_deposit);
    }
}

#[test]
fn test_multi_arbitrator_quorum_voting() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        _arbitrator,
        token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);

    let job_id = String::from_str(&env, "job-multi-arb");
    let timeout = env.ledger().timestamp() + 10_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arb1.clone(),
    };

    escrow_client.create_escrow(&job_id, &client, &params, &Vec::new(&env));
    escrow_client.start_work(&job_id, &freelancer);
    escrow_client.raise_dispute(&job_id, &client);

    // Initial balances
    let client_initial = token_client.balance(&client);
    let freelancer_initial = token_client.balance(&freelancer);

    // Vote 1: arb1 votes (700k to client, 300k to freelancer)
    let res1 = escrow_client.vote_dispute(&job_id, &arb1, &700_000, &300_000);
    assert_eq!(res1, false); // Quorum not reached yet

    let votes = escrow_client.get_dispute_votes(&job_id);
    assert_eq!(votes.len(), 1);

    // Vote 2: arb2 votes different split (500k / 500k)
    let res2 = escrow_client.vote_dispute(&job_id, &arb2, &500_000, &500_000);
    assert_eq!(res2, false); // Different vote, quorum still not reached

    // Vote 3: arb3 votes matching arb1 (700k / 300k) -> 2 of 3 quorum reached!
    let res3 = escrow_client.vote_dispute(&job_id, &arb3, &700_000, &300_000);
    assert_eq!(res3, true); // Quorum reached and dispute resolved!

    // Verify final status and payout
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);

    let client_gain = token_client.balance(&client) - client_initial;
    let freelancer_gain = token_client.balance(&freelancer) - freelancer_initial;

    // Fee: 1% of 300,000 = 3,000
    assert_eq!(client_gain, 700_000);
    assert_eq!(freelancer_gain, 297_000);
    assert_eq!(token_client.balance(&treasury), 3_000);
}

#[test]
fn test_milestone_deliverable_submission_and_query() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-ms-deliverable");
    let timeout = env.ledger().timestamp() + 10_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator,
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Architecture Design"),
        percentage: 40,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Implementation"),
        percentage: 60,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    // Freelancer submits deliverable for milestone 1
    let hash_m1 = BytesN::from_array(&env, &[0x42; 32]);
    escrow_client.submit_milestone_deliverable(&job_id, &freelancer, &1, &hash_m1);

    // Verify hash query matches
    let registered_hash = escrow_client.get_milestone_deliverable(&job_id, &1);
    assert_eq!(registered_hash, hash_m1);

    // Verify querying milestone 2 before submission fails
    let res_m2 = escrow_client.try_get_milestone_deliverable(&job_id, &2);
    assert_eq!(res_m2, Err(Ok(EscrowError::MilestoneNotFound)));
}

#[test]
fn test_arbitrator_registration_and_staking_bond() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        _client,
        _freelancer,
        arbitrator,
        token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    // Initial bond is 0
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 0);

    // Mint tokens to arbitrator for staking
    token_admin.mint(&arbitrator, &500_000);

    // Arbitrator deposits 300,000 bond
    escrow_client.register_arbitrator(&arbitrator, &token_address, &300_000);

    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 300_000);
    assert_eq!(token_client.balance(&arbitrator), 200_000);

    // Arbitrator tops up another 100,000 bond
    escrow_client.register_arbitrator(&arbitrator, &token_address, &100_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 400_000);
}

#[test]
fn test_arbitrator_bond_slashing_mechanism() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    // Mint tokens to arbitrator
    token_admin.mint(&arbitrator, &500_000);
    escrow_client.register_arbitrator(&arbitrator, &token_address, &500_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 500_000);

    let client_initial = token_client.balance(&client);

    // Non-admin attempting to slash fails
    let non_admin_res = escrow_client.try_slash_arbitrator_bond(
        &freelancer,
        &arbitrator,
        &client,
        &token_address,
        &100_000,
    );
    assert_eq!(non_admin_res, Err(Ok(EscrowError::Unauthorized)));

    // Slashing more than available bond fails
    let excess_res = escrow_client.try_slash_arbitrator_bond(
        &admin,
        &arbitrator,
        &client,
        &token_address,
        &600_000,
    );
    assert_eq!(excess_res, Err(Ok(EscrowError::InsufficientBond)));

    // Admin slashes 200,000 bond to client
    escrow_client.slash_arbitrator_bond(
        &admin,
        &arbitrator,
        &client,
        &token_address,
        &200_000,
    );

    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 300_000);
    assert_eq!(token_client.balance(&client) - client_initial, 200_000);
}

#[test]
fn test_dispute_evidence_submission_and_query() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-evidence-test");
    let timeout = env.ledger().timestamp() + 10_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: 500_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Full Job"),
        percentage: 100,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    // Cannot submit evidence before dispute is raised
    let client_hash = BytesN::from_array(&env, &[0xaa; 32]);
    let pre_dispute_res = escrow_client.try_submit_dispute_evidence(&job_id, &client, &client_hash);
    assert_eq!(pre_dispute_res, Err(Ok(EscrowError::EscrowNotDisputed)));

    // Client raises dispute
    escrow_client.raise_dispute(&job_id, &client);

    // Client submits evidence
    escrow_client.submit_dispute_evidence(&job_id, &client, &client_hash);
    assert_eq!(escrow_client.get_dispute_evidence(&job_id, &client), client_hash);

    // Freelancer submits rebuttal evidence
    let freelancer_hash = BytesN::from_array(&env, &[0xbb; 32]);
    escrow_client.submit_dispute_evidence(&job_id, &freelancer, &freelancer_hash);
    assert_eq!(escrow_client.get_dispute_evidence(&job_id, &freelancer), freelancer_hash);

    // Unauthorized non-party fails
    let outsider = Address::generate(&env);
    let outsider_res = escrow_client.try_submit_dispute_evidence(&job_id, &outsider, &client_hash);
    assert_eq!(outsider_res, Err(Ok(EscrowError::Unauthorized)));
}

#[test]
fn test_milestone_deadlines_and_overdue_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-deadline-test");
    let base_time = env.ledger().timestamp();
    let timeout = base_time + 50_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 1"),
        percentage: 50,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 2"),
        percentage: 50,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    // Initial check: no deadline set
    assert_eq!(escrow_client.get_milestone_deadline(&job_id, &1), None);
    assert_eq!(escrow_client.is_milestone_expired(&job_id, &1), false);

    // Setting past deadline fails
    let past_res = escrow_client.try_set_milestone_deadline(&job_id, &client, &1, &base_time);
    assert_eq!(past_res, Err(Ok(EscrowError::InvalidDeadline)));

    // Setting valid future deadline succeeds
    let target_deadline = base_time + 3600;
    escrow_client.set_milestone_deadline(&job_id, &client, &1, &target_deadline);
    assert_eq!(escrow_client.get_milestone_deadline(&job_id, &1), Some(target_deadline));
    assert_eq!(escrow_client.is_milestone_expired(&job_id, &1), false);

    // Advance time past deadline
    env.ledger().set_timestamp(target_deadline + 10);
    assert_eq!(escrow_client.is_milestone_expired(&job_id, &1), true);

    // After approval, milestone is no longer considered expired
    escrow_client.release_milestone(&job_id, &client, &1);
    assert_eq!(escrow_client.is_milestone_expired(&job_id, &1), false);
}
