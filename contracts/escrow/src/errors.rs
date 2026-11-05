use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    EscrowAlreadyExists = 4,
    EscrowNotFound = 5,
    InvalidStatus = 6,
    InvalidAmount = 7,
    InvalidMilestonePercentages = 8,
    TimeoutNotExpired = 9,
    TimeoutAlreadyExpired = 10,
    MilestoneAlreadyApproved = 11,
    MilestoneNotFound = 12,
    InvalidDeliverableHash = 13,
    DisputeAlreadyRaised = 14,
    EscrowNotDisputed = 15,
}

