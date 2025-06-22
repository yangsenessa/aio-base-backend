use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use candid::Principal;
use num_traits::ToPrimitive;
use crate::mining_reword::{RewardEntry, UserRewardKey};

type Memory = VirtualMemory<DefaultMemoryImpl>;

pub const AIO_MINING_POOL_ID: &str = "cmx4w-ltfgv-strkj-zbcjj-ulg3p-e2rsl-haeth-le6mq-47xa4-reygn-iqe";
//pub const AIO_MINING_POOL_ID: &str = "6nimk-xpves-34bk3-zf7dp-nykqv-h3ady-iu3ze-xplot-vm4uy-ptbel-3qe";

pub const TOKEN_LEDGER_CANISTER_ID: &str = "mxzaz-hqaaa-aaaar-qaada-cai";

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
    pub updated_at: Option<u64>,
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
                token_balance: 0u64,
                credit_balance: 0u64,
                staked_credits: 0u64,
                kappa_multiplier: 1.0,
            },
            created_at: ic_cdk::api::time(),
            updated_at: None,
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

// Create a wrapper type for Vec<u64>
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

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CreditConvertContract {
    pub price_credits: f64, // Dollar price per Credit
    pub price_icp: f64,     // Current ICP dollar price
}

impl ic_stable_structures::Storable for CreditConvertContract {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode CreditConvertContract"))
    }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode CreditConvertContract")
    }
    const BOUND: Bound = Bound::Bounded { max_size: 128, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RechargeRecord {
    pub user: Principal,
    pub icp_amount: f64,
    pub credits_obtained: u64,
    pub timestamp: u64,
}

impl ic_stable_structures::Storable for RechargeRecord {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode RechargeRecord"))
    }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode RechargeRecord")
    }
    const BOUND: Bound = Bound::Bounded { max_size: 128, is_fixed_size: false };
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RechargePrincipalAccount {
    pub principal_id: String, // Principal id of the user who recharges
    pub subaccount_id: Option<String>,   // Target subaccount id for recharge
}

impl ic_stable_structures::Storable for RechargePrincipalAccount {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode RechargePrincipalAccount"))
    }
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode RechargePrincipalAccount")
    }
    const BOUND: Bound = Bound::Bounded { max_size: 512, is_fixed_size: false };
}

