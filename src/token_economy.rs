use candid::{CandidType, Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::StableBTreeMap;
use std::cell::RefCell;
use crate::token_economy_types::{
    EmissionPolicy, SubscriptionPlan, TokenGrant, TokenGrantKey,
    TokenActivity, TokenActivityType, CreditActivity, CreditActivityType,
    TransferStatus, AccountInfo
};
use icrc_ledger_types::{icrc1::account::Account, icrc1::transfer::{TransferError, BlockIndex}};
use crate::trace_storage::get_trace;
use crate::account_storage::{get_account, upsert_account};
use std::collections::HashMap;
use crate::token_economy_types::*;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::DefaultMemoryImpl;
use std::borrow::Cow;
use serde::{Serialize, Deserialize};

// Re-export NumTokens for public use
pub use icrc_ledger_types::icrc1::transfer::NumTokens;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TransferResult {
    pub success: bool,
    pub error: Option<TransferError>,
    pub block_height: Option<u64>,
}

type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenAccountKey {
    pub principal_id: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenAccount {
    pub principal_id: String,
    pub aio_balance: u64,
    pub credit_balance: u64,
    pub staked_credits: u64,
    pub staking_start_time: Option<u64>,
    pub subscription_plan: Option<SubscriptionPlan>,
    pub last_claim_time: Option<u64>,
    pub kappa_multiplier: f64,
}

impl ic_stable_structures::Storable for TokenAccountKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.principal_id).expect("Failed to encode TokenAccountKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let principal_id = Decode!(bytes.as_ref(), String).expect("Failed to decode TokenAccountKey");
        Self { principal_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for TokenAccount {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode TokenAccount"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode TokenAccount")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 32, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static TOKEN_ACCOUNTS: RefCell<StableBTreeMap<TokenAccountKey, TokenAccount, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(23)))
        )
    );
}

// Constants
const EXCHANGE_RATIO: f64 = 1.0; // 1 AIO = 1 Credit
const STAKING_PERIOD: u64 = 30 * 24 * 60 * 60 * 1_000_000_000; // 30 days in nanoseconds
const MIN_STAKE_AMOUNT: u64 = 100; // Minimum amount of credits to stake
const MAX_KAPPA: f64 = 2.0; // Maximum kappa multiplier
const BASE_KAPPA: f64 = 1.0; // Base kappa multiplier
const DEFAULT_BASE_RATE: u64 = 100;
const DEFAULT_KAPPA_FACTOR: f64 = 1.0;
const DEFAULT_STAKING_BONUS: f64 = 0.1;

// Exchange Module
pub fn convert_aio_to_credits(principal_id: String, amount: u64) -> Result<u64, String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id: principal_id.clone() };
        
        let mut account = accounts.get(&key).unwrap_or_else(|| TokenAccount {
            principal_id: principal_id.clone(),
            aio_balance: 0,
            credit_balance: 0,
            staked_credits: 0,
            staking_start_time: None,
            subscription_plan: None,
            last_claim_time: None,
            kappa_multiplier: BASE_KAPPA,
        });

        if account.aio_balance < amount {
            return Err("Insufficient AIO balance".to_string());
        }

        let credits = (amount as f64 * EXCHANGE_RATIO) as u64;
        account.aio_balance -= amount;
        account.credit_balance += credits;
        
        accounts.insert(key, account);
        Ok(credits)
    })
}

pub fn update_exchange_ratio(new_ratio: f64) -> Result<(), String> {
    if new_ratio <= 0.0 {
        return Err("Exchange ratio must be positive".to_string());
    }
    // In a real implementation, this would update a global exchange ratio
    Ok(())
}

// Subscription Module
pub fn subscribe_plan(principal_id: String, plan: SubscriptionPlan) -> Result<(), String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id: principal_id.clone() };
        
        let mut account = accounts.get(&key).unwrap_or_else(|| TokenAccount {
            principal_id: principal_id.clone(),
            aio_balance: 0,
            credit_balance: 0,
            staked_credits: 0,
            staking_start_time: None,
            subscription_plan: None,
            last_claim_time: None,
            kappa_multiplier: BASE_KAPPA,
        });

        account.subscription_plan = Some(plan);
        accounts.insert(key, account);
        Ok(())
    })
}

// Credit Staking
pub fn stack_credit(principal_id: String, amount: u64) -> Result<(), String> {
    if amount < MIN_STAKE_AMOUNT {
        return Err(format!("Minimum stake amount is {}", MIN_STAKE_AMOUNT));
    }

    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id: principal_id.clone() };
        
        let mut account = accounts.get(&key).unwrap_or_else(|| TokenAccount {
            principal_id: principal_id.clone(),
            aio_balance: 0,
            credit_balance: 0,
            staked_credits: 0,
            staking_start_time: None,
            subscription_plan: None,
            last_claim_time: None,
            kappa_multiplier: BASE_KAPPA,
        });

        if account.credit_balance < amount {
            return Err("Insufficient credit balance".to_string());
        }

        account.credit_balance -= amount;
        account.staked_credits += amount;
        account.staking_start_time = Some(time());
        
        accounts.insert(key, account);
        Ok(())
    })
}

// κ Multiplier
pub fn get_kappa(principal_id: String) -> Result<f64, String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        let key = TokenAccountKey { principal_id };
        
        if let Some(account) = accounts.get(&key) {
            Ok(account.kappa_multiplier)
        } else {
            Ok(BASE_KAPPA)
        }
    })
}

// Reward Claim
pub fn claim_reward(principal_id: String) -> Result<u64, String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id: principal_id.clone() };
        
        let mut account = accounts.get(&key).unwrap_or_else(|| TokenAccount {
            principal_id: principal_id.clone(),
            aio_balance: 0,
            credit_balance: 0,
            staked_credits: 0,
            staking_start_time: None,
            subscription_plan: None,
            last_claim_time: None,
            kappa_multiplier: BASE_KAPPA,
        });

        if account.staked_credits == 0 {
            return Err("No staked credits to claim rewards from".to_string());
        }

        let staking_start = account.staking_start_time.ok_or("No staking start time")?;
        let current_time = time();
        
        if current_time - staking_start < STAKING_PERIOD {
            return Err("Staking period not completed".to_string());
        }

        // Calculate rewards based on staked amount and kappa
        let reward = (account.staked_credits as f64 * account.kappa_multiplier * 0.1) as u64;
        
        // Update account
        account.credit_balance += reward;
        account.staked_credits = 0;
        account.staking_start_time = None;
        account.last_claim_time = Some(current_time);
        
        accounts.insert(key, account);
        Ok(reward)
    })
}

// Initialize emission policy
pub fn init_emission_policy() {
    let mut policy = EmissionPolicy {
        base_rate: DEFAULT_BASE_RATE,
        kappa_factor: DEFAULT_KAPPA_FACTOR,
        staking_bonus: DEFAULT_STAKING_BONUS,
        subscription_multipliers: HashMap::new(),
        last_update_time: time(),
    };

    // Set default subscription multipliers
    policy.subscription_multipliers.insert(SubscriptionPlan::Free, 1.0);
    policy.subscription_multipliers.insert(SubscriptionPlan::Basic, 1.5);
    policy.subscription_multipliers.insert(SubscriptionPlan::Premium, 2.0);
    policy.subscription_multipliers.insert(SubscriptionPlan::Enterprise, 3.0);

    crate::token_economy_types::EMISSION_POLICY.with(|p| {
        p.borrow_mut().insert("default".to_string(), policy);
    });
}

// Calculate token emission for an account
pub fn calculate_emission(principal_id: &str) -> Result<u64, String> {
    let account = get_account(principal_id.to_string())
        .ok_or_else(|| "Account not found".to_string())?;
    let policy = get_emission_policy()?;
    
    let base_amount = policy.base_rate;
    let kappa_multiplier = account.get_kappa_multiplier();
    let staked_credits = account.get_staked_credits();
    let staking_bonus = if staked_credits > 0 {
        policy.staking_bonus
    } else {
        1.0
    };

    let subscription_multiplier = account.get_subscription_plan()
        .and_then(|plan| policy.subscription_multipliers.get(&plan))
        .copied()
        .unwrap_or(1.0);

    let emission = (base_amount as f64 * kappa_multiplier * staking_bonus * subscription_multiplier) as u64;
    Ok(emission)
}

// Get emission policy
pub fn get_emission_policy() -> Result<EmissionPolicy, String> {
    crate::token_economy_types::EMISSION_POLICY.with(|p| {
        p.borrow()
            .get(&"default".to_string())
            .ok_or_else(|| "Emission policy not found".to_string())
    })
}

// Update emission policy
pub fn update_emission_policy(policy: EmissionPolicy) -> Result<(), String> {
    crate::token_economy_types::EMISSION_POLICY.with(|p| {
        p.borrow_mut().insert("default".to_string(), policy);
        Ok(())
    })
}

// Create token grant
pub fn create_token_grant(grant: TokenGrant) -> Result<(), String> {
    let key = TokenGrantKey {
        recipient: grant.recipient.clone(),
    };

    crate::token_economy_types::TOKEN_GRANTS.with(|g| {
        g.borrow_mut().insert(key, grant);
        Ok(())
    })
}

// Get token grant
pub fn get_token_grant(recipient: &str) -> Result<TokenGrant, String> {
    let key = TokenGrantKey {
        recipient: recipient.to_string(),
    };

    crate::token_economy_types::TOKEN_GRANTS.with(|g| {
        g.borrow()
            .get(&key)
            .ok_or_else(|| "Token grant not found".to_string())
    })
}

// Claim vested tokens
pub fn claim_vested_tokens(principal_id: &str) -> Result<u64, String> {
    let grant = get_token_grant(principal_id)?;
    let current_time = time();

    if current_time < grant.start_time {
        return Err("Vesting period has not started".to_string());
    }

    let elapsed_time = current_time - grant.start_time;
    let vested_amount = if elapsed_time >= grant.vesting_period {
        grant.amount - grant.claimed_amount
    } else {
        (grant.amount as f64 * (elapsed_time as f64 / grant.vesting_period as f64)) as u64 - grant.claimed_amount
    };

    if vested_amount == 0 {
        return Err("No tokens available to claim".to_string());
    }

    // Update grant
    let mut updated_grant = grant.clone();
    updated_grant.claimed_amount += vested_amount;
    create_token_grant(updated_grant)?;

    // Update account balance
    let mut account = get_account(principal_id.to_string())
        .ok_or_else(|| "Account not found".to_string())?;
    account.balance += vested_amount;
    upsert_account(account)?;

    // Record token activity
    let activity = TokenActivity {
        timestamp: current_time,
        from: "system".to_string(),
        to: principal_id.to_string(),
        amount: vested_amount,
        activity_type: TokenActivityType::Vest,
        status: TransferStatus::Completed,
        metadata: Some("Vested token claim".to_string()),
    };
    record_token_activity(activity)?;

    Ok(vested_amount)
}

// Helper functions for recording activities
pub fn record_token_activity(activity: TokenActivity) -> Result<(), String> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        let mut activities = activities.borrow_mut();
        let index = activities.len();
        activities.insert(index, activity);
        Ok(())
    })
}

pub fn record_credit_activity(activity: CreditActivity) -> Result<(), String> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        let mut activities = activities.borrow_mut();
        let index = activities.len();
        activities.insert(index, activity);
        Ok(())
    })
}

// Get all token grants
pub fn get_all_token_grants() -> Vec<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|g| {
        g.borrow().iter().map(|(_, grant)| grant.clone()).collect()
    })
}

// Get account token information
pub fn get_account_token_info(principal_id: &str) -> Result<(u64, u64, f64), String> {
    let account = get_account(principal_id.to_string())
        .ok_or_else(|| "Account not found".to_string())?;
    let staked_credits = account.get_staked_credits();
    let kappa_multiplier = account.get_kappa_multiplier();
    let balance = account.balance;

    Ok((balance, staked_credits, kappa_multiplier))
}

// Ledger Utility Tracker
pub fn log_credit_usage(principal_id: String, amount: u64, service: String, metadata: Option<String>) -> Result<(), String> {
    let activity = CreditActivity {
        timestamp: time(),
        principal_id: principal_id.clone(),
        amount,
        activity_type: CreditActivityType::Spend,
        status: TransferStatus::Completed,
        metadata: Some(format!("Credit usage for service: {} - {}", service, metadata.unwrap_or_default())),
    };
    record_credit_activity(activity)
}

pub fn log_credit_utility(principal_id: String, amount: u64, service: String, metadata: Option<String>) -> Result<(), String> {
    log_credit_usage(principal_id, amount, service, metadata)
}

// Query methods for TokenActivity
pub fn get_token_activities(principal_id: &str) -> Vec<TokenActivity> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.from == principal_id || activity.to == principal_id)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_token_activities_paginated(principal_id: &str, offset: u64, limit: usize) -> Vec<TokenActivity> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.from == principal_id || activity.to == principal_id)
            .skip(offset as usize)
            .take(limit)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_token_activities_by_type(principal_id: &str, activity_type: TokenActivityType) -> Vec<TokenActivity> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| 
                (activity.from == principal_id || activity.to == principal_id) && 
                activity.activity_type == activity_type
            )
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_token_activities_by_time_period(principal_id: &str, start_time: u64, end_time: u64) -> Vec<TokenActivity> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| 
                (activity.from == principal_id || activity.to == principal_id) && 
                activity.timestamp >= start_time && 
                activity.timestamp <= end_time
            )
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

// Query methods for CreditActivity
pub fn get_credit_activities(principal_id: &str) -> Vec<CreditActivity> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.principal_id == principal_id)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_credit_activities_paginated(principal_id: &str, offset: u64, limit: usize) -> Vec<CreditActivity> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.principal_id == principal_id)
            .skip(offset as usize)
            .take(limit)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_credit_activities_by_type(principal_id: &str, activity_type: CreditActivityType) -> Vec<CreditActivity> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| 
                activity.principal_id == principal_id && 
                activity.activity_type == activity_type
            )
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_credit_activities_by_time_period(principal_id: &str, start_time: u64, end_time: u64) -> Vec<CreditActivity> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| 
                activity.principal_id == principal_id && 
                activity.timestamp >= start_time && 
                activity.timestamp <= end_time
            )
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

// Statistics methods
pub fn get_token_activity_statistics(principal_id: &str) -> (u64, u64, u64) {
    let activities = get_token_activities(principal_id);
    let total_count = activities.len() as u64;
    let total_amount = activities.iter().map(|a| a.amount).sum();
    let success_count = activities.iter()
        .filter(|a| matches!(a.status, TransferStatus::Completed))
        .count() as u64;
    
    (total_count, total_amount, success_count)
}

pub fn get_credit_activity_statistics(principal_id: &str) -> (u64, u64, u64) {
    let activities = get_credit_activities(principal_id);
    let total_count = activities.len() as u64;
    let total_amount = activities.iter().map(|a| a.amount).sum();
    let success_count = activities.iter()
        .filter(|a| matches!(a.status, TransferStatus::Completed))
        .count() as u64;
    
    (total_count, total_amount, success_count)
}

/// Get account information
pub fn get_account_info(principal_id: String) -> Option<TokenAccount> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        let key = TokenAccountKey { principal_id };
        accounts.get(&key)
    })
}

/// Add a new account
pub fn add_account(principal_id: String, symbol: String) -> Result<TokenAccount, String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id: principal_id.clone() };
        
        if accounts.contains_key(&key) {
            return Err("Account already exists".to_string());
        }

        let account = TokenAccount {
            principal_id: principal_id.clone(),
            aio_balance: 0,
            credit_balance: 0,
            staked_credits: 0,
            staking_start_time: None,
            subscription_plan: None,
            last_claim_time: None,
            kappa_multiplier: BASE_KAPPA,
        };
        
        accounts.insert(key, account.clone());
        Ok(account)
    })
}

/// Get all accounts
pub fn get_all_accounts() -> Vec<TokenAccount> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        accounts.iter().map(|(_, account)| account).collect()
    })
}

/// Get accounts with pagination
pub fn get_accounts_paginated(offset: u64, limit: usize) -> Vec<TokenAccount> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        accounts.iter()
            .skip(offset as usize)
            .take(limit)
            .map(|(_, account)| account)
            .collect()
    })
}

/// Delete an account
pub fn delete_account(principal_id: String) -> Result<(), String> {
    TOKEN_ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = TokenAccountKey { principal_id };
        if accounts.remove(&key).is_some() {
            Ok(())
        } else {
            Err("Account not found".to_string())
        }
    })
}

/// Get balance summary
pub fn get_balance_summary(principal_id: String) -> (u64, u64, u64, u64) {
    if let Some(account) = get_account_info(principal_id) {
        (
            account.aio_balance,
            account.staked_credits,
            account.credit_balance,
            0 // unclaimed_balance is not part of TokenAccount
        )
    } else {
        (0, 0, 0, 0)
    }
} 