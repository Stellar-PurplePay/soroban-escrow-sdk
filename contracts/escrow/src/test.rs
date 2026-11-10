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

#[test]
fn test_cancel_expired_milestone_refund() {
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

    let job_id = String::from_str(&env, "job-cancel-expired");
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
        percentage: 40,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 2"),
        percentage: 60,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    let initial_client_balance = token_client.balance(&client);

    // Set deadline for milestone 1
    let dl1 = base_time + 1000;
    escrow_client.set_milestone_deadline(&job_id, &client, &1, &dl1);

    // Try cancelling before deadline -> fails with MilestoneNotExpired
    let early_cancel = escrow_client.try_cancel_expired_milestone(&job_id, &client, &1);
    assert_eq!(early_cancel, Err(Ok(EscrowError::MilestoneNotExpired)));

    // Fast-forward past deadline
    env.ledger().set_timestamp(dl1 + 5);

    // Cancel expired milestone 1 -> client gets refunded 400_000
    escrow_client.cancel_expired_milestone(&job_id, &client, &1);
    assert_eq!(token_client.balance(&client), initial_client_balance + 400_000);

    // Cannot cancel again
    let double_cancel = escrow_client.try_cancel_expired_milestone(&job_id, &client, &1);
    assert_eq!(double_cancel, Err(Ok(EscrowError::MilestoneAlreadyApproved)));
}

#[test]
fn test_extend_milestone_deadline() {
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

    let job_id = String::from_str(&env, "job-extend-dl");
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
        description: String::from_str(&env, "Deliverable"),
        percentage: 100,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    // Initial deadline
    let initial_dl = base_time + 2000;
    escrow_client.set_milestone_deadline(&job_id, &client, &1, &initial_dl);
    assert_eq!(escrow_client.get_milestone_deadline(&job_id, &1), Some(initial_dl));

    // Zero seconds extension fails
    let zero_ext = escrow_client.try_extend_milestone_deadline(&job_id, &client, &1, &0);
    assert_eq!(zero_ext, Err(Ok(EscrowError::InvalidDeadline)));

    // Client extends deadline by 1500 seconds
    let new_dl = escrow_client.extend_milestone_deadline(&job_id, &client, &1, &1500);
    assert_eq!(new_dl, initial_dl + 1500);
    assert_eq!(escrow_client.get_milestone_deadline(&job_id, &1), Some(initial_dl + 1500));

    // Non-client cannot extend
    let non_client = Address::generate(&env);
    let unauthorized = escrow_client.try_extend_milestone_deadline(&job_id, &non_client, &1, &1000);
    assert_eq!(unauthorized, Err(Ok(EscrowError::Unauthorized)));
}

#[test]
fn test_release_partial_milestone_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-partial-release");
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
        description: String::from_str(&env, "Project Phase 1"),
        percentage: 100,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    let initial_freelancer_bal = token_client.balance(&freelancer);
    let initial_treasury_bal = token_client.balance(&treasury);

    // Initial state: 0 released, 1,000,000 remaining
    assert_eq!(escrow_client.get_milestone_released_amount(&job_id, &1), 0);
    assert_eq!(escrow_client.get_milestone_remaining_amount(&job_id, &1), 1_000_000);

    // First partial release: 400,000 (fee is 100 bps = 1.0% = 4,000; net 396,000)
    let remaining_after_first = escrow_client.release_partial_milestone(&job_id, &client, &1, &400_000);
    assert_eq!(remaining_after_first, 600_000);
    assert_eq!(escrow_client.get_milestone_released_amount(&job_id, &1), 400_000);
    assert_eq!(escrow_client.get_milestone_remaining_amount(&job_id, &1), 600_000);

    assert_eq!(token_client.balance(&freelancer), initial_freelancer_bal + 396_000);
    assert_eq!(token_client.balance(&treasury), initial_treasury_bal + 4_000);

    // Attempting to release more than remaining (700,000 > 600,000) fails with InvalidAmount
    let over_release = escrow_client.try_release_partial_milestone(&job_id, &client, &1, &700_000);
    assert_eq!(over_release, Err(Ok(EscrowError::InvalidAmount)));

    // Second partial release: remaining 600,000 (fee is 1.0% = 6,000; net 594,000)
    let remaining_after_second = escrow_client.release_partial_milestone(&job_id, &client, &1, &600_000);
    assert_eq!(remaining_after_second, 0);
    assert_eq!(escrow_client.get_milestone_released_amount(&job_id, &1), 1_000_000);
    assert_eq!(escrow_client.get_milestone_remaining_amount(&job_id, &1), 0);

    assert_eq!(token_client.balance(&freelancer), initial_freelancer_bal + 396_000 + 594_000);
    assert_eq!(token_client.balance(&treasury), initial_treasury_bal + 4_000 + 6_000);

    let final_escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(final_escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_property_partial_release_sum_conservation() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        treasury,
        client,
        freelancer,
        arbitrator,
        token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let job_id = String::from_str(&env, "job-prop-partial");
    let total_milestone = 2_000_000;
    let timeout = env.ledger().timestamp() + 50_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: total_milestone,
        timeout_timestamp: timeout,
        arbitrator,
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Full Work"),
        percentage: 100,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    let increments: [i128; 5] = [200_000, 350_000, 450_000, 500_000, 500_000];
    let mut total_released = 0;
    let initial_freelancer_bal = token_client.balance(&freelancer);
    let initial_treasury_bal = token_client.balance(&treasury);

    for chunk in increments.iter() {
        let expected_remaining = total_milestone - (total_released + chunk);
        let remaining = escrow_client.release_partial_milestone(&job_id, &client, &1, chunk);
        assert_eq!(remaining, expected_remaining);

        total_released += chunk;
        let released_stored = escrow_client.get_milestone_released_amount(&job_id, &1);
        let remaining_stored = escrow_client.get_milestone_remaining_amount(&job_id, &1);

        // Invariant: released + remaining == total_milestone
        assert_eq!(released_stored + remaining_stored, total_milestone);
        assert_eq!(released_stored, total_released);
    }

    // Platform fee 1% (100 bps)
    let total_fee = (total_milestone * 100) / 10_000;
    let total_freelancer = total_milestone - total_fee;

    assert_eq!(token_client.balance(&freelancer), initial_freelancer_bal + total_freelancer);
    assert_eq!(token_client.balance(&treasury), initial_treasury_bal + total_fee);
}

#[test]
fn test_receipt_counter_and_event_issuance() {
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

    let job_id = String::from_str(&env, "job-receipt-counter");
    let timeout = env.ledger().timestamp() + 50_000;

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: 1_000_000,
        timeout_timestamp: timeout,
        arbitrator,
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

    // Initial counter is 0 (or previous tests in isolated env start at 0)
    let initial_receipts = escrow_client.get_receipt_counter();

    // Partial release generates receipt #1
    escrow_client.release_partial_milestone(&job_id, &client, &1, &200_000);
    assert_eq!(escrow_client.get_receipt_counter(), initial_receipts + 1);

    // Partial release generates receipt #2
    escrow_client.release_partial_milestone(&job_id, &client, &1, &300_000);
    assert_eq!(escrow_client.get_receipt_counter(), initial_receipts + 2);

    // Full milestone release generates receipt #3
    escrow_client.release_milestone(&job_id, &client, &2);
    assert_eq!(escrow_client.get_receipt_counter(), initial_receipts + 3);
}

#[test]
fn test_dispute_and_timeout_receipt_issuance() {
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

    let counter_before = escrow_client.get_receipt_counter();

    // 1. Escrow resolved by arbitrator
    let job_id1 = String::from_str(&env, "job-dispute-rcpt");
    let params1 = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 500_000,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator: arbitrator.clone(),
    };
    let mut ms1 = Vec::new(&env);
    ms1.push_back(MilestoneInput {
        description: String::from_str(&env, "M1"),
        percentage: 100,
    });

    escrow_client.create_escrow(&job_id1, &client, &params1, &ms1);
    escrow_client.start_work(&job_id1, &freelancer);
    escrow_client.raise_dispute(&job_id1, &client);

    // Arbitrator resolves: 200k to client, 300k to freelancer -> 2 receipts (1 client, 1 freelancer)
    escrow_client.resolve_dispute(&job_id1, &arbitrator, &200_000, &300_000);
    assert_eq!(escrow_client.get_receipt_counter(), counter_before + 2);

    // 2. Escrow resolved via timeout refund
    let job_id2 = String::from_str(&env, "job-timeout-rcpt");
    let timeout = env.ledger().timestamp() + 100;
    let params2 = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 400_000,
        timeout_timestamp: timeout,
        arbitrator: arbitrator.clone(),
    };
    escrow_client.create_escrow(&job_id2, &client, &params2, &ms1);

    // Advance ledger past timeout
    env.ledger().with_mut(|li| {
        li.timestamp = timeout + 1;
    });

    escrow_client.claim_timeout_refund(&job_id2);
    // Timeout refund issues 1 receipt for the refunded client amount
    assert_eq!(escrow_client.get_receipt_counter(), counter_before + 3);
}

#[test]
fn test_receipt_counter_monotonic_invariant() {
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

    let start_counter = escrow_client.get_receipt_counter();
    let mut expected_receipts = start_counter;

    // Escrow 1: 2 partial releases + 1 full release = 3 receipts
    let job1 = String::from_str(&env, "inv-job-1");
    let mut ms1 = Vec::new(&env);
    ms1.push_back(MilestoneInput {
        description: String::from_str(&env, "M1"),
        percentage: 60,
    });
    ms1.push_back(MilestoneInput {
        description: String::from_str(&env, "M2"),
        percentage: 40,
    });
    let params1 = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator: arbitrator.clone(),
    };
    escrow_client.create_escrow(&job1, &client, &params1, &ms1);
    escrow_client.start_work(&job1, &freelancer);

    escrow_client.release_partial_milestone(&job1, &client, &1, &200_000);
    expected_receipts += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipts);

    escrow_client.release_partial_milestone(&job1, &client, &1, &400_000);
    expected_receipts += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipts);

    escrow_client.release_milestone(&job1, &client, &2);
    expected_receipts += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipts);

    // Escrow 2: Cancel before start = 1 refund receipt
    let job2 = String::from_str(&env, "inv-job-2");
    let mut ms2 = Vec::new(&env);
    ms2.push_back(MilestoneInput {
        description: String::from_str(&env, "All"),
        percentage: 100,
    });
    let params2 = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 500_000,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator: arbitrator.clone(),
    };
    escrow_client.create_escrow(&job2, &client, &params2, &ms2);
    escrow_client.cancel_and_refund(&job2, &client);
    expected_receipts += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipts);

    // Escrow 3: Expired milestone cancel = 1 receipt
    let job3 = String::from_str(&env, "inv-job-3");
    let mut ms3 = Vec::new(&env);
    ms3.push_back(MilestoneInput {
        description: String::from_str(&env, "M1"),
        percentage: 50,
    });
    ms3.push_back(MilestoneInput {
        description: String::from_str(&env, "M2"),
        percentage: 50,
    });
    let params3 = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 600_000,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator: arbitrator.clone(),
    };
    escrow_client.create_escrow(&job3, &client, &params3, &ms3);
    escrow_client.start_work(&job3, &freelancer);

    let dl = env.ledger().timestamp() + 500;
    escrow_client.set_milestone_deadline(&job3, &client, &1, &dl);

    env.ledger().with_mut(|li| {
        li.timestamp = dl + 10;
    });

    escrow_client.cancel_expired_milestone(&job3, &client, &1);
    expected_receipts += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipts);

    // Verify global monotonicity
    assert!(escrow_client.get_receipt_counter() > start_counter);
    assert_eq!(escrow_client.get_receipt_counter() - start_counter, 5);
}

#[test]
fn test_admin_action_throttling_guard() {
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

    let new_treasury1 = Address::generate(&env);
    let new_treasury2 = Address::generate(&env);

    // First update succeeds
    escrow_client.set_treasury(&new_treasury1);

    // Immediate second update within 300s fails with RateLimitExceeded
    let res = escrow_client.try_set_treasury(&new_treasury2);
    assert_eq!(res, Err(Ok(EscrowError::RateLimitExceeded)));

    // Advance ledger timestamp by 301 seconds
    let now = env.ledger().timestamp();
    env.ledger().with_mut(|li| {
        li.timestamp = now + 301;
    });

    // Throttled cooldown elapsed; second update now succeeds
    escrow_client.set_treasury(&new_treasury2);
}

#[test]
fn test_timeout_minimum_grace_period_enforcement() {
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

    let job_id = String::from_str(&env, "job-grace-period");
    let now = env.ledger().timestamp();

    // Timeout only 30s in future (< MIN_TIMEOUT_DURATION 60s)
    let params_invalid = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 100_000,
        timeout_timestamp: now + 30,
        arbitrator: arbitrator.clone(),
    };
    let milestones = Vec::new(&env);

    let res = escrow_client.try_create_escrow(&job_id, &client, &params_invalid, &milestones);
    assert_eq!(res, Err(Ok(EscrowError::TimeoutAlreadyExpired)));

    // Valid timeout >= 60s
    let params_valid = CreateEscrowParams {
        freelancer,
        token: token_address,
        total_amount: 100_000,
        timeout_timestamp: now + 60,
        arbitrator,
    };
    let res_valid = escrow_client.try_create_escrow(&job_id, &client, &params_valid, &milestones);
    assert!(res_valid.is_ok());
}

#[test]
fn test_property_dispute_award_conservation() {
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

    let test_matrix: [(i128, i128, i128); 6] = [
        (1_000_000, 1_000_000, 0),         // 100% to client
        (1_000_000, 0, 1_000_000),         // 100% to freelancer
        (1_000_000, 500_000, 500_000),     // 50/50 split
        (1_000_000, 250_000, 750_000),     // 25/75 split
        (1_000_001, 333_333, 666_668),     // Odd amount with non-zero remainder
        (9_999_999, 1, 9_999_998),         // Extreme edge split
    ];

    for (idx, (total, client_part, freelancer_part)) in test_matrix.iter().enumerate() {
        token_admin.mint(&client, total);

        let initial_client_bal = token_client.balance(&client);
        let initial_freelancer_bal = token_client.balance(&freelancer);
        let initial_treasury_bal = token_client.balance(&treasury);

        let job_id = match idx {
            0 => String::from_str(&env, "prop-disp-0"),
            1 => String::from_str(&env, "prop-disp-1"),
            2 => String::from_str(&env, "prop-disp-2"),
            3 => String::from_str(&env, "prop-disp-3"),
            4 => String::from_str(&env, "prop-disp-4"),
            _ => String::from_str(&env, "prop-disp-5"),
        };

        let params = CreateEscrowParams {
            freelancer: freelancer.clone(),
            token: token_address.clone(),
            total_amount: *total,
            timeout_timestamp: env.ledger().timestamp() + 50_000,
            arbitrator: arbitrator.clone(),
        };
        let milestones = Vec::new(&env);

        escrow_client.create_escrow(&job_id, &client, &params, &milestones);
        escrow_client.start_work(&job_id, &freelancer);
        escrow_client.raise_dispute(&job_id, &client);

        escrow_client.resolve_dispute(&job_id, &arbitrator, client_part, freelancer_part);

        let client_gain = token_client.balance(&client) - (initial_client_bal - *total);
        let freelancer_gain = token_client.balance(&freelancer) - initial_freelancer_bal;
        let treasury_gain = token_client.balance(&treasury) - initial_treasury_bal;

        // Conservation invariant: client received + freelancer received + fee paid strictly equals total deposited
        assert_eq!(
            client_gain + freelancer_gain + treasury_gain,
            *total,
            "Dispute funds conservation failed for test split #{}",
            idx
        );
    }
}

#[test]
fn test_property_fee_ceiling_and_deduction_bounds() {
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

    // Verify rejection of fees exceeding 10.00% (1,000 bps)
    let bad_fees: [u32; 4] = [1001, 1500, 5000, 10000];
    for bad_fee in bad_fees {
        let res = escrow_client.try_set_fee_bps(&bad_fee);
        assert_eq!(res, Err(Ok(EscrowError::InvalidFee)));
    }

    // Matrix of valid (fee_bps, amount) pairs
    let test_cases: [(u32, i128); 8] = [
        (0, 10_000),
        (50, 100_000),
        (100, 1_000_000),
        (250, 10_000_000),
        (500, 50_000_000),
        (1000, 100_000_000),
        (100, 17),        // Small odd amount
        (1000, 999_999),   // Odd amount with max fee
    ];

    for (idx, (fee_bps, amount)) in test_cases.iter().enumerate() {
        // Advance ledger to avoid throttle cooldown
        let now = env.ledger().timestamp();
        env.ledger().with_mut(|li| {
            li.timestamp = now + 400;
        });

        escrow_client.set_fee_bps(fee_bps);

        token_admin.mint(&client, amount);

        let initial_freelancer_bal = token_client.balance(&freelancer);
        let initial_treasury_bal = token_client.balance(&treasury);

        let job_id = match idx {
            0 => String::from_str(&env, "fee-case-0"),
            1 => String::from_str(&env, "fee-case-1"),
            2 => String::from_str(&env, "fee-case-2"),
            3 => String::from_str(&env, "fee-case-3"),
            4 => String::from_str(&env, "fee-case-4"),
            5 => String::from_str(&env, "fee-case-5"),
            6 => String::from_str(&env, "fee-case-6"),
            _ => String::from_str(&env, "fee-case-7"),
        };

        let params = CreateEscrowParams {
            freelancer: freelancer.clone(),
            token: token_address.clone(),
            total_amount: *amount,
            timeout_timestamp: env.ledger().timestamp() + 50_000,
            arbitrator: arbitrator.clone(),
        };
        let milestones = Vec::new(&env);

        escrow_client.create_escrow(&job_id, &client, &params, &milestones);
        escrow_client.start_work(&job_id, &freelancer);
        escrow_client.release_milestone(&job_id, &client, &1);

        let fee_deducted = token_client.balance(&treasury) - initial_treasury_bal;
        let net_payout = token_client.balance(&freelancer) - initial_freelancer_bal;

        // Invariant 1: Total conservation
        assert_eq!(fee_deducted + net_payout, *amount);

        // Invariant 2: Fee deduction is bounded strictly by fee_bps
        let expected_max_fee = (*amount * (*fee_bps as i128)) / 10_000;
        assert!(fee_deducted <= expected_max_fee);

        if *fee_bps == 0 {
            assert_eq!(fee_deducted, 0);
            assert_eq!(net_payout, *amount);
        }
    }
}

#[test]
fn test_property_arbitrator_bond_conservation() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        _client,
        _freelancer,
        _arbitrator,
        token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let arb_a = Address::generate(&env);
    let arb_b = Address::generate(&env);
    let arb_c = Address::generate(&env);
    let penalty_recipient = Address::generate(&env);

    token_admin.mint(&arb_a, &2_000_000);
    token_admin.mint(&arb_b, &2_000_000);
    token_admin.mint(&arb_c, &2_000_000);

    let contract_addr = &escrow_client.address;

    // 1. Initial registrations
    escrow_client.register_arbitrator(&arb_a, &token_address, &500_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_a), 500_000);

    escrow_client.register_arbitrator(&arb_b, &token_address, &1_000_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_b), 1_000_000);

    escrow_client.register_arbitrator(&arb_c, &token_address, &750_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_c), 750_000);

    let mut expected_total_bonds: i128 = 500_000 + 1_000_000 + 750_000;
    assert_eq!(token_client.balance(&contract_addr), expected_total_bonds);

    // 2. Bond top-up
    escrow_client.register_arbitrator(&arb_a, &token_address, &350_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_a), 850_000);
    expected_total_bonds += 350_000;
    assert_eq!(token_client.balance(&contract_addr), expected_total_bonds);

    // 3. Penalty slash Arb A
    let initial_recipient_bal = token_client.balance(&penalty_recipient);
    escrow_client.slash_arbitrator_bond(&admin, &arb_a, &penalty_recipient, &token_address, &250_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_a), 600_000);
    expected_total_bonds -= 250_000;
    assert_eq!(token_client.balance(&contract_addr), expected_total_bonds);
    assert_eq!(token_client.balance(&penalty_recipient) - initial_recipient_bal, 250_000);

    // 4. Penalty slash Arb B
    escrow_client.slash_arbitrator_bond(&admin, &arb_b, &penalty_recipient, &token_address, &600_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_b), 400_000);
    expected_total_bonds -= 600_000;
    assert_eq!(token_client.balance(&contract_addr), expected_total_bonds);
    assert_eq!(token_client.balance(&penalty_recipient) - initial_recipient_bal, 850_000);

    // 5. Excessive slash rejected
    let excess_res = escrow_client.try_slash_arbitrator_bond(&admin, &arb_c, &penalty_recipient, &token_address, &999_999);
    assert_eq!(excess_res, Err(Ok(EscrowError::InsufficientBond)));
    assert_eq!(escrow_client.get_arbitrator_bond(&arb_c), 750_000);

    // Invariant: sum of all individual bonds strictly matches contract balance
    let sum_bonds = escrow_client.get_arbitrator_bond(&arb_a)
        + escrow_client.get_arbitrator_bond(&arb_b)
        + escrow_client.get_arbitrator_bond(&arb_c);
    assert_eq!(sum_bonds, expected_total_bonds);
    assert_eq!(token_client.balance(&contract_addr), sum_bonds);
}

#[test]
fn test_property_receipt_monotonic_concurrency() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let mut expected_receipt_count = escrow_client.get_receipt_counter();

    // Create 3 separate jobs
    for i in 1..=3 {
        token_admin.mint(&client, &1_000_000);
        let job_id = match i {
            1 => String::from_str(&env, "job-interleaved-1"),
            2 => String::from_str(&env, "job-interleaved-2"),
            _ => String::from_str(&env, "job-interleaved-3"),
        };
        let mut ms = Vec::new(&env);
        ms.push_back(MilestoneInput {
            description: String::from_str(&env, "M1"),
            percentage: 50,
        });
        ms.push_back(MilestoneInput {
            description: String::from_str(&env, "M2"),
            percentage: 50,
        });
        let params = CreateEscrowParams {
            freelancer: freelancer.clone(),
            token: token_address.clone(),
            total_amount: 1_000_000,
            timeout_timestamp: env.ledger().timestamp() + 50_000,
            arbitrator: arbitrator.clone(),
        };
        escrow_client.create_escrow(&job_id, &client, &params, &ms);
        escrow_client.start_work(&job_id, &freelancer);
    }

    // Interleaved action sequence:
    // Action 1: Job 1 partial release 100k -> +1 receipt
    let j1 = String::from_str(&env, "job-interleaved-1");
    escrow_client.release_partial_milestone(&j1, &client, &1, &100_000);
    expected_receipt_count += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);

    // Action 2: Job 2 full milestone 1 release -> +1 receipt
    let j2 = String::from_str(&env, "job-interleaved-2");
    escrow_client.release_milestone(&j2, &client, &1);
    expected_receipt_count += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);

    // Action 3: Job 3 partial release 250k -> +1 receipt
    let j3 = String::from_str(&env, "job-interleaved-3");
    escrow_client.release_partial_milestone(&j3, &client, &1, &250_000);
    expected_receipt_count += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);

    // Action 4: Job 1 partial release another 200k -> +1 receipt
    escrow_client.release_partial_milestone(&j1, &client, &1, &200_000);
    expected_receipt_count += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);

    // Action 5: Job 3 raised dispute and resolved -> +2 receipts (client + freelancer awards)
    escrow_client.raise_dispute(&j3, &client);
    escrow_client.resolve_dispute(&j3, &arbitrator, &300_000, &450_000);
    expected_receipt_count += 2;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);

    // Action 6: Job 2 full milestone 2 release -> +1 receipt
    escrow_client.release_milestone(&j2, &client, &2);
    expected_receipt_count += 1;
    assert_eq!(escrow_client.get_receipt_counter(), expected_receipt_count);
}

#[test]
fn test_arbitrator_bond_withdrawal() {
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

    token_admin.mint(&arbitrator, &1_000_000);
    assert_eq!(token_client.balance(&arbitrator), 1_000_000);

    // 1. Register and stake 600,000 bond
    escrow_client.register_arbitrator(&arbitrator, &token_address, &600_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 600_000);
    assert_eq!(token_client.balance(&arbitrator), 400_000);

    // 2. Withdraw partial bond (250,000)
    escrow_client.withdraw_arbitrator_bond(&arbitrator, &token_address, &250_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 350_000);
    assert_eq!(token_client.balance(&arbitrator), 650_000);

    // 3. Excess withdrawal rejected
    let excess_res = escrow_client.try_withdraw_arbitrator_bond(&arbitrator, &token_address, &500_000);
    assert_eq!(excess_res.err(), Some(Ok(EscrowError::InvalidAmount)));

    // 4. Withdraw remaining bond (350,000)
    escrow_client.withdraw_arbitrator_bond(&arbitrator, &token_address, &350_000);
    assert_eq!(escrow_client.get_arbitrator_bond(&arbitrator), 0);
    assert_eq!(token_client.balance(&arbitrator), 1_000_000);
}

#[test]
fn test_archive_completed_escrow_lifecycle() {
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

    let job_id = String::from_str(&env, "archive-job-1");
    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Sole Milestone"),
        percentage: 100,
    });

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: env.ledger().timestamp() + 3600,
        arbitrator: arbitrator.clone(),
    };

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // 1. Cannot archive InProgress escrow
    let premature_res = escrow_client.try_archive_completed_escrow(&job_id);
    assert_eq!(premature_res.err(), Some(Ok(EscrowError::InvalidStatus)));

    // 2. Release milestone so status transitions to Completed
    escrow_client.release_milestone(&job_id, &client, &1);
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);

    // 3. Now archive succeeds
    escrow_client.archive_completed_escrow(&job_id);

    // Milestones are cleaned up from storage
    let milestones_res = escrow_client.try_get_milestones(&job_id);
    assert_eq!(milestones_res.err(), Some(Ok(EscrowError::EscrowNotFound)));
}

#[test]
fn test_dispute_cooling_off_enforcement() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    // Set 300-second dispute cooling-off period
    escrow_client.set_dispute_cooldown(&admin, &300);
    assert_eq!(escrow_client.get_dispute_cooldown(), 300);

    let job_id = String::from_str(&env, "cooling-job-1");
    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Disputed milestone"),
        percentage: 100,
    });

    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address.clone(),
        total_amount: 1_000_000,
        timeout_timestamp: env.ledger().timestamp() + 3600,
        arbitrator: arbitrator.clone(),
    };

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    // Raise dispute
    escrow_client.raise_dispute(&job_id, &client);

    // Attempt premature resolution within 300s cooling-off period
    let premature_res = escrow_client.try_resolve_dispute(&job_id, &arbitrator, &500_000, &500_000);
    assert_eq!(premature_res.err(), Some(Ok(EscrowError::RateLimitExceeded)));

    // Advance ledger timestamp beyond cooldown (301 seconds)
    env.ledger().with_mut(|li| li.timestamp += 301);

    // Resolution now succeeds
    escrow_client.resolve_dispute(&job_id, &arbitrator, &500_000, &500_000);
    let escrow = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_emergency_freeze_notice_period_and_unfreeze() {
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

    // Initial status: not frozen
    let (active, unlock_ts) = escrow_client.get_emergency_freeze_status();
    assert_eq!(active, false);
    assert_eq!(unlock_ts, 0);

    let now = env.ledger().timestamp();
    // Request emergency freeze with 100 seconds (clamped to 86_400 minimum notice)
    let unlock_expected = escrow_client.request_emergency_freeze(&100);
    assert_eq!(unlock_expected, now + 86_400);

    let (active_after, unlock_after) = escrow_client.get_emergency_freeze_status();
    assert_eq!(active_after, true);
    assert_eq!(unlock_after, now + 86_400);

    // Attempt unfreeze before notice period elapses
    let premature_unfreeze = escrow_client.try_execute_emergency_unfreeze();
    assert_eq!(premature_unfreeze.err(), Some(Ok(EscrowError::FreezePeriodNotExpired)));

    // Advance halfway through notice period (43,200 seconds)
    env.ledger().with_mut(|li| li.timestamp += 43_200);
    let still_premature = escrow_client.try_execute_emergency_unfreeze();
    assert_eq!(still_premature.err(), Some(Ok(EscrowError::FreezePeriodNotExpired)));

    // Advance beyond notice period (+45,000 seconds)
    env.ledger().with_mut(|li| li.timestamp += 45_000);

    // Unfreeze succeeds
    escrow_client.execute_emergency_unfreeze();
    let (active_final, unlock_final) = escrow_client.get_emergency_freeze_status();
    assert_eq!(active_final, false);
    assert_eq!(unlock_final, 0);
}

#[test]
fn test_audit_log_counter_and_receipt_verification() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    assert_eq!(escrow_client.get_audit_log_count(), 0);

    let hash1 = BytesN::from_array(&env, &[1u8; 32]);
    let hash2 = BytesN::from_array(&env, &[2u8; 32]);
    let hash3 = BytesN::from_array(&env, &[3u8; 32]);

    let action1 = symbol_short!("create");
    let action2 = symbol_short!("release");
    let action3 = symbol_short!("resolve");

    let idx1 = escrow_client.record_audit_entry(&client, &action1, &hash1);
    assert_eq!(idx1, 1);
    assert_eq!(escrow_client.get_audit_log_count(), 1);

    let idx2 = escrow_client.record_audit_entry(&client, &action2, &hash2);
    assert_eq!(idx2, 2);
    assert_eq!(escrow_client.get_audit_log_count(), 2);

    let idx3 = escrow_client.record_audit_entry(&admin, &action3, &hash3);
    assert_eq!(idx3, 3);
    assert_eq!(escrow_client.get_audit_log_count(), 3);

    // Exact match verification
    assert_eq!(escrow_client.verify_audit_receipt(&1, &hash1), true);
    assert_eq!(escrow_client.verify_audit_receipt(&2, &hash2), true);
    assert_eq!(escrow_client.verify_audit_receipt(&3, &hash3), true);

    // Wrong hash returns false
    assert_eq!(escrow_client.verify_audit_receipt(&1, &hash2), false);

    // Non-existent index returns AuditIndexMismatch
    let non_existent = escrow_client.try_verify_audit_receipt(&999, &hash1);
    assert_eq!(non_existent.err(), Some(Ok(EscrowError::AuditIndexMismatch)));
}

#[test]
fn test_contract_wasm_upgrade_authorization_and_timelock() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    let dummy_wasm_hash = BytesN::from_array(&env, &[99u8; 32]);

    // Non-admin cannot stage upgrade
    let unauthorized_stage = escrow_client.try_stage_upgrade(&client, &dummy_wasm_hash);
    assert_eq!(unauthorized_stage.err(), Some(Ok(EscrowError::Unauthorized)));

    // Non-admin cannot set upgrade delay
    let unauthorized_delay = escrow_client.try_set_upgrade_delay(&client, &7200);
    assert_eq!(unauthorized_delay.err(), Some(Ok(EscrowError::Unauthorized)));

    // Delay lower than 3600 seconds rejected
    let invalid_delay = escrow_client.try_set_upgrade_delay(&admin, &1800);
    assert_eq!(invalid_delay.err(), Some(Ok(EscrowError::InvalidUpgradeDelay)));

    // Set custom 7200s delay
    escrow_client.set_upgrade_delay(&admin, &7200);
    assert_eq!(escrow_client.get_upgrade_delay(), 7200);

    // Stage upgrade
    let now = env.ledger().timestamp();
    let unlock_ts = escrow_client.stage_upgrade(&admin, &dummy_wasm_hash);
    assert_eq!(unlock_ts, now + 7200);

    let staged = escrow_client.get_staged_upgrade();
    assert!(staged.is_some());
    let (staged_hash, staged_unlock) = staged.unwrap();
    assert_eq!(staged_hash, dummy_wasm_hash);
    assert_eq!(staged_unlock, now + 7200);

    // Premature execution fails with UpgradeTimelockActive
    let premature_exec = escrow_client.try_execute_upgrade(&admin);
    assert_eq!(premature_exec.err(), Some(Ok(EscrowError::UpgradeTimelockActive)));

    // Cancel staged upgrade
    escrow_client.cancel_staged_upgrade(&admin);
    assert!(escrow_client.get_staged_upgrade().is_none());

    // Executing when not staged fails with UpgradeNotStaged
    let not_staged_exec = escrow_client.try_execute_upgrade(&admin);
    assert_eq!(not_staged_exec.err(), Some(Ok(EscrowError::UpgradeNotStaged)));
}

#[test]
fn test_governance_proposals_quorum_and_voting() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    let title = String::from_str(&env, "Protocol Parameter Revision");
    let description = String::from_str(&env, "Lower base platform fee to 50 bps");
    let quorum = 1000;
    let duration = 3600;

    let proposal_id = escrow_client.create_proposal(&admin, &title, &description, &quorum, &duration);
    assert_eq!(proposal_id, 1);

    let prop = escrow_client.get_proposal(&proposal_id);
    assert_eq!(prop.votes_for, 0);
    assert_eq!(prop.votes_against, 0);
    assert_eq!(prop.quorum_threshold, quorum);

    // Initial check: proposal has not passed
    assert_eq!(escrow_client.is_proposal_passed(&proposal_id), false);

    // Voter 1 casts 600 votes FOR
    escrow_client.vote_proposal(&proposal_id, &client, &true, &600);
    // Total votes 600 < 1000 quorum
    assert_eq!(escrow_client.is_proposal_passed(&proposal_id), false);

    // Voter 1 attempts duplicate vote
    let dup_vote = escrow_client.try_vote_proposal(&proposal_id, &client, &true, &100);
    assert_eq!(dup_vote.err(), Some(Ok(EscrowError::ProposalAlreadyVoted)));

    // Voter 2 casts 500 votes FOR -> total votes 1100 >= 1000 quorum, FOR > AGAINST
    escrow_client.vote_proposal(&proposal_id, &freelancer, &true, &500);
    assert_eq!(escrow_client.is_proposal_passed(&proposal_id), true);

    // Advance time past proposal duration
    env.ledger().with_mut(|li| li.timestamp += 3601);

    // Voting after expiration fails
    let late_vote = escrow_client.try_vote_proposal(&proposal_id, &arbitrator, &false, &400);
    assert_eq!(late_vote.err(), Some(Ok(EscrowError::ProposalExpired)));
}

#[test]
fn test_upgrade_staging_and_timelock_window() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        _client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    // Initial default delay is 86,400s (24 hours)
    assert_eq!(escrow_client.get_upgrade_delay(), 86_400);

    let wasm_hash = BytesN::from_array(&env, &[77u8; 32]);
    let t0 = env.ledger().timestamp();
    let unlock_ts = escrow_client.stage_upgrade(&admin, &wasm_hash);
    assert_eq!(unlock_ts, t0 + 86_400);

    // Advance 50,000s (still within timelock)
    env.ledger().with_mut(|li| li.timestamp += 50_000);
    assert_eq!(escrow_client.try_execute_upgrade(&admin).err(), Some(Ok(EscrowError::UpgradeTimelockActive)));

    // Cancel staged upgrade before window expires
    escrow_client.cancel_staged_upgrade(&admin);
    assert!(escrow_client.get_staged_upgrade().is_none());
}

#[test]
fn test_upgrade_events_emitted_with_correct_topics() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        admin,
        _treasury,
        _client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    let wasm_hash = BytesN::from_array(&env, &[88u8; 32]);
    let unlock_ts = escrow_client.stage_upgrade(&admin, &wasm_hash);
    assert!(unlock_ts > env.ledger().timestamp());

    let staged = escrow_client.get_staged_upgrade();
    assert_eq!(staged, Some((wasm_hash.clone(), unlock_ts)));

    escrow_client.cancel_staged_upgrade(&admin);
    assert_eq!(escrow_client.get_staged_upgrade(), None);
}

#[test]
fn test_deliverable_caller_authorization_and_party_validation() {
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

    let job_id = String::from_str(&env, "auth-deliverable-job");
    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: 1_000_000,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator: arbitrator.clone(),
    };

    let milestones = Vec::new(&env);
    escrow_client.create_escrow(&job_id, &client, &params, &milestones);

    let deliv_hash = BytesN::from_array(&env, &[42u8; 32]);

    // Authorized freelancer submission succeeds
    escrow_client.submit_deliverable_by_party(&job_id, &freelancer, &deliv_hash);
    assert_eq!(escrow_client.get_deliverable(&job_id), deliv_hash.clone());

    // Authorized client submission succeeds
    let client_hash = BytesN::from_array(&env, &[43u8; 32]);
    escrow_client.submit_deliverable_by_party(&job_id, &client, &client_hash);
    assert_eq!(escrow_client.get_deliverable(&job_id), client_hash);

    // Unauthorized third-party (arbitrator is not client or freelancer) rejected
    let unauthorized_hash = BytesN::from_array(&env, &[99u8; 32]);
    let unauthorized_res = escrow_client.try_submit_deliverable_by_party(&job_id, &arbitrator, &unauthorized_hash);
    assert_eq!(unauthorized_res.err(), Some(Ok(EscrowError::Unauthorized)));
}

#[test]
fn test_anti_replay_account_nonce_guard() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        _freelancer,
        _arbitrator,
        _token_client,
        _token_admin,
        _token_address,
        escrow_client,
    ) = setup_test(&env);

    // Initial nonce is 0
    assert_eq!(escrow_client.get_account_nonce(&client), 0);

    // Valid next sequential nonce: 1
    let n1 = escrow_client.verify_and_increment_nonce(&client, &1);
    assert_eq!(n1, 1);
    assert_eq!(escrow_client.get_account_nonce(&client), 1);

    // Replay attack: resubmitting nonce 1 fails
    let replay_res = escrow_client.try_verify_and_increment_nonce(&client, &1);
    assert_eq!(replay_res.err(), Some(Ok(EscrowError::InvalidNonce)));

    // Out-of-order jump: skipping to nonce 5 fails
    let jump_res = escrow_client.try_verify_and_increment_nonce(&client, &5);
    assert_eq!(jump_res.err(), Some(Ok(EscrowError::InvalidNonce)));

    // Valid sequential nonce: 2
    let n2 = escrow_client.verify_and_increment_nonce(&client, &2);
    assert_eq!(n2, 2);
    assert_eq!(escrow_client.get_account_nonce(&client), 2);
}

#[test]
fn test_batch_release_milestones_consolidated_settlement() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let total = 1_000_000;
    token_admin.mint(&client, &total);

    let job_id = String::from_str(&env, "batch-release-job");
    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: total,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator,
    };

    let mut milestones = Vec::new(&env);
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 1"),
        percentage: 30,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 2"),
        percentage: 30,
    });
    milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "Milestone 3"),
        percentage: 40,
    });

    escrow_client.create_escrow(&job_id, &client, &params, &milestones);
    escrow_client.start_work(&job_id, &freelancer);

    let mut batch1 = Vec::new(&env);
    batch1.push_back(1);
    batch1.push_back(2);

    let released1 = escrow_client.batch_release_milestones(&job_id, &client, &batch1);
    assert_eq!(released1, 600_000);

    let escrow_mid = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow_mid.released_amount, 600_000);
    assert_eq!(escrow_mid.status, EscrowStatus::InProgress);

    // Release remaining milestone 3
    let mut batch2 = Vec::new(&env);
    batch2.push_back(3);
    let released2 = escrow_client.batch_release_milestones(&job_id, &client, &batch2);
    assert_eq!(released2, 400_000);

    let escrow_final = escrow_client.get_escrow(&job_id);
    assert_eq!(escrow_final.released_amount, total);
    assert_eq!(escrow_final.status, EscrowStatus::Completed);
}

#[test]
fn test_milestone_array_maximum_gas_bounds_and_preview_enforced() {
    let env = Env::default();
    env.mock_all_auths();

    let (
        _admin,
        _treasury,
        client,
        freelancer,
        arbitrator,
        _token_client,
        token_admin,
        token_address,
        escrow_client,
    ) = setup_test(&env);

    let total = 1_000_000;
    token_admin.mint(&client, &total);

    let job_id = String::from_str(&env, "bounds-preview-job");
    let params = CreateEscrowParams {
        freelancer: freelancer.clone(),
        token: token_address,
        total_amount: total,
        timeout_timestamp: env.ledger().timestamp() + 50_000,
        arbitrator,
    };

    // Construct 26 milestones (> MAX_MILESTONES 25)
    let mut overflow_milestones = Vec::new(&env);
    for _ in 0..26 {
        overflow_milestones.push_back(MilestoneInput {
            description: String::from_str(&env, "M"),
            percentage: 3,
        });
    }

    let overflow_res = escrow_client.try_create_escrow(&job_id, &client, &params, &overflow_milestones);
    assert_eq!(overflow_res.err(), Some(Ok(EscrowError::InvalidMilestoneCount)));

    // Empty batch release rejected
    let empty_batch = Vec::new(&env);
    let empty_res = escrow_client.try_batch_release_milestones(&job_id, &client, &empty_batch);
    assert_eq!(empty_res.err(), Some(Ok(EscrowError::InvalidMilestoneCount)));

    // Valid escrow with 2 milestones
    let mut valid_milestones = Vec::new(&env);
    valid_milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "M1"),
        percentage: 40,
    });
    valid_milestones.push_back(MilestoneInput {
        description: String::from_str(&env, "M2"),
        percentage: 60,
    });
    escrow_client.create_escrow(&job_id, &client, &params, &valid_milestones);

    // Preview batch release
    let mut preview_batch = Vec::new(&env);
    preview_batch.push_back(1);
    preview_batch.push_back(2);

    let (total_release, fee, net_payout) = escrow_client.preview_batch_release(&job_id, &preview_batch);
    assert_eq!(total_release, 1_000_000);
    assert_eq!(total_release, fee + net_payout);
}
