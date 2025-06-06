use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use candid::Principal;

type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TokenActivityType {
    Transfer,
    Stack,
    Unstack,
    Claim,
    Grant,
    Vest,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenActivity {
    pub timestamp: u64,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub activity_type: TokenActivityType,
    pub status: TransferStatus,
    pub metadata: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CreditActivityType {
    Earn,
    Spend,
    Stack,
    Unstack,
    Reward,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CreditActivity {
    pub timestamp: u64,
    pub principal_id: String,
    pub amount: u64,
    pub activity_type: CreditActivityType,
    pub status: TransferStatus,
    pub metadata: Option<String>,
}

// Token Economy Types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SubscriptionPlan {
    Free,
    Basic,
    Premium,
    Enterprise,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct EmissionPolicy {
    pub base_rate: u64,
    pub kappa_factor: f64,
    pub staking_bonus: f64,
    pub subscription_multipliers: HashMap<SubscriptionPlan, f64>,
    pub last_update_time: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct GrantPolicy {
    pub grant_amount: u64,
    pub grant_action: GrantAction,
    pub grant_duration: u64,

}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrantAction {
    NewUser,
    NewMcp
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TokenGrantStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenGrant {
    pub recipient: String,
    pub amount: u64,
    pub start_time: u64,
    pub claimed_amount: u64,
    pub status: TokenGrantStatus,
}
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct NewMcpGrant {
    pub recipient: String,
    pub amount: u64,
    pub start_time: u64,
    pub claimed_amount: u64,
    pub mcp_name:String,
    pub status: TokenGrantStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, CandidType, Serialize, Deserialize)]
pub struct NewMcpGrantKey {
    pub recipient: String,
    pub mcp_name: String
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenInfo {
    pub token_balance: u64,
    pub credit_balance: u64,
    pub staked_credits: u64,
    pub kappa_multiplier: f64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AccountInfo {
    pub principal_id: String,
    pub token_info: TokenInfo,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: Option<String>,
}

impl AccountInfo {
    pub fn get_subscription_plan(&self) -> Option<SubscriptionPlan> {
        // Parse from metadata or dedicated field
        None // TODO: Implement based on your storage strategy
    }

    pub fn get_kappa_multiplier(&self) -> f64 {
        self.token_info.kappa_multiplier
    }

    pub fn get_staked_credits(&self) -> u64 {
        self.token_info.staked_credits
    }

    pub fn get_token_balance(&self) -> u64 {
        self.token_info.token_balance
    }

    pub fn get_credit_balance(&self) -> u64 {
        self.token_info.credit_balance
    }

    pub fn new(principal_id: String) -> Self {
        Self {
            principal_id,
            token_info: TokenInfo {
                token_balance: 0,
                credit_balance: 0,
                staked_credits: 0,
                kappa_multiplier: 1.0,
            },
            created_at: ic_cdk::api::time(),
            updated_at: ic_cdk::api::time(),
            metadata: None,
        }
    }
}

// Implement Storable for EmissionPolicy
impl ic_stable_structures::Storable for EmissionPolicy {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode EmissionPolicy"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode EmissionPolicy")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for TokenGrant
impl ic_stable_structures::Storable for TokenGrant {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode TokenGrant"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode TokenGrant")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Define the key for token grant data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenGrantKey {
    pub recipient: String,
}

impl ic_stable_structures::Storable for TokenGrantKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.recipient).expect("Failed to encode TokenGrantKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let recipient = Decode!(bytes.as_ref(), String).expect("Failed to decode TokenGrantKey");
        Self { recipient }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

// Implement Storable for new types
impl ic_stable_structures::Storable for TokenActivity {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode TokenActivity"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode TokenActivity")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

impl ic_stable_structures::Storable for CreditActivity {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode CreditActivity"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode CreditActivity")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for GrantAction
impl ic_stable_structures::Storable for GrantAction {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode GrantAction"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode GrantAction")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

// Implement Storable for GrantPolicy
impl ic_stable_structures::Storable for GrantPolicy {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode GrantPolicy"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode GrantPolicy")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for NewMcpGrant
impl ic_stable_structures::Storable for NewMcpGrant {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode NewMcpGrant"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode NewMcpGrant")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for NewMcpGrantKey
impl ic_stable_structures::Storable for NewMcpGrantKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.recipient.as_bytes());
        bytes.push(0); // null terminator
        bytes.extend_from_slice(self.mcp_name.as_bytes());
        bytes.push(0); // null terminator
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut parts = bytes.split(|&b| b == 0);
        let recipient = String::from_utf8(parts.next().unwrap_or_default().to_vec())
            .unwrap_or_default();
        let mcp_name = String::from_utf8(parts.next().unwrap_or_default().to_vec())
            .unwrap_or_default();
        Self { recipient, mcp_name }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TraceItem {
    pub context_id: String,
    pub trace_id: String,
    pub owner: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub calls: Vec<CallItem>,
    pub metadata: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CallItem {
    pub id: u64,
    pub protocol: String,
    pub agent: String,
    pub call_type: String,
    pub method: String,
    pub inputs: Vec<IOData>,
    pub outputs: Vec<IOData>,
    pub status: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct IOData {
    pub data_type: String,
    pub value: String,
}

// Mining reward types
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct QuarterRewardConfig {
    pub quarter: u32,
    pub base_reward: u64,
    pub estimated_calls: u64,
    pub total_quarter_emission: u64,
}

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct MiningRewardPolicy {
    pub start_epoch_sec: u64,
    pub decay_rate: f32,
    pub total_emission_cap: u64,
    pub quarters: Vec<QuarterRewardConfig>,
}

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct RewardEntry {
    pub principal_id: Principal,
    pub mcp_name: String,
    pub reward_amount: u64,
    pub block_id: u64,
    pub status: String,
}

#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserRewardKey {
    pub principal_id: Principal,
    pub mcp_name: String,
}

// Implement Storable for QuarterRewardConfig
impl ic_stable_structures::Storable for QuarterRewardConfig {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode QuarterRewardConfig"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode QuarterRewardConfig")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for MiningRewardPolicy
impl ic_stable_structures::Storable for MiningRewardPolicy {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode MiningRewardPolicy"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode MiningRewardPolicy")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for RewardEntry
impl ic_stable_structures::Storable for RewardEntry {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode RewardEntry"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode RewardEntry")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// Implement Storable for UserRewardKey
impl ic_stable_structures::Storable for UserRewardKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode UserRewardKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode UserRewardKey")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

// 为 Vec<u64> 创建一个包装类型
#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct RewardIdList(pub Vec<u64>);

impl ic_stable_structures::Storable for RewardIdList {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode RewardIdList"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode RewardIdList")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

