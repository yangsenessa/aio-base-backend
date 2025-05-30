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
    NewDeveloper
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

// Initialize stable memory storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    pub static EMISSION_POLICY: RefCell<StableBTreeMap<String, EmissionPolicy, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(21)))
        )
    );
    
    pub static TOKEN_GRANTS: RefCell<StableBTreeMap<TokenGrantKey, TokenGrant, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(24)))
        )
    );

    pub static TOKEN_ACTIVITIES: RefCell<StableBTreeMap<u64, TokenActivity, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(24)))
        )
    );

    pub static CREDIT_ACTIVITIES: RefCell<StableBTreeMap<u64, CreditActivity, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(25)))
        )
    );

    pub static GRANT_POLICIES: RefCell<StableBTreeMap<GrantAction, GrantPolicy, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(26)))
        )
    );
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