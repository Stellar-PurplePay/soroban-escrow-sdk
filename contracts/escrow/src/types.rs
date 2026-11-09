use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Created,
    InProgress,
    Completed,
    Refunded,
    Disputed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub percentage: u32,
    pub amount: i128,
    pub approved: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub job_id: String,
    pub client: Address,
    pub freelancer: Address,
    pub token: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub timeout_timestamp: u64,
    pub arbitrator: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateEscrowParams {
    pub freelancer: Address,
    pub token: Address,
    pub total_amount: i128,
    pub timeout_timestamp: u64,
    pub arbitrator: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneInput {
    pub description: String,
    pub percentage: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowProgress {
    pub total_amount: i128,
    pub released_amount: i128,
    pub percentage_released: u32,
    pub status: EscrowStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitratorVote {
    pub arbitrator: Address,
    pub client_award: i128,
    pub freelancer_award: i128,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Treasury,
    FeeBps,
    Paused,
    Escrow(String),
    Milestones(String),
    Deliverable(String),
    MilestoneDeliverable(String, u32),
    DisputeBallot(String),
    ArbitratorBond(Address),
    DisputeEvidence(String, Address),
    MilestoneDeadline(String, u32),
    MilestoneReleased(String, u32),
}
