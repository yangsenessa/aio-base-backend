use candid::{CandidType, Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::StableBTreeMap;
use std::cell::RefCell;
use crate::token_economy_types::{
    EmissionPolicy, SubscriptionPlan, TokenGrant, TokenGrantKey,
    TokenActivity, TokenActivityType, CreditActivity, CreditActivityType,
    TransferStatus, AccountInfo, TokenInfo, TokenGrantStatus
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

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Constants
const EXCHANGE_RATIO: f64 = 1.0; // 1 AIO = 1 Credit
const STAKING_PERIOD: u64 = 30 * 24 * 60 * 60 * 1_000_000_000; // 30 days in nanoseconds
const MIN_STAKE_AMOUNT: u64 = 100; // Minimum amount of credits to stake
const MAX_KAPPA: f64 = 2.0; // Maximum kappa multiplier
const BASE_KAPPA: f64 = 1.0; // Base kappa multiplier
const DEFAULT_BASE_RATE: u64 = 100;
const DEFAULT_KAPPA_FACTOR: f64 = 1.0;
const DEFAULT_STAKING_BONUS: f64 = 0.1;

// Account Management
pub fn get_account_info(principal_id: String) -> Option<AccountInfo> {
    get_account(principal_id)
}

pub fn create_account(principal_id: String) -> Result<AccountInfo, String> {
    let account = AccountInfo::new(principal_id);
    upsert_account(account)
}

pub fn update_account_balance(principal_id: String, token_amount: i64, credit_amount: i64) -> Result<AccountInfo, String> {
    let mut account = get_account(principal_id.clone())
        .ok_or_else(|| "Account not found".to_string())?;

    if token_amount < 0 && account.get_token_balance() < (-token_amount) as u64 {
        return Err("Insufficient token balance".to_string());
    }
    if credit_amount < 0 && account.get_credit_balance() < (-credit_amount) as u64 {
        return Err("Insufficient credit balance".to_string());
    }

    account.token_info.token_balance = (account.get_token_balance() as i64 + token_amount) as u64;
    account.token_info.credit_balance = (account.get_credit_balance() as i64 + credit_amount) as u64;
    account.updated_at = time();

    upsert_account(account)
}

// Credit Operations
pub fn stack_credits(principal_id: String, amount: u64) -> Result<AccountInfo, String> {
    if amount < MIN_STAKE_AMOUNT {
        return Err(format!("Minimum stake amount is {}", MIN_STAKE_AMOUNT));
    }

    let mut account = get_account(principal_id.clone())
        .ok_or_else(|| "Account not found".to_string())?;

    if account.get_credit_balance() < amount {
        return Err("Insufficient credit balance".to_string());
    }

    account.token_info.credit_balance -= amount;
    account.token_info.staked_credits += amount;
    account.updated_at = time();
    
    let result = upsert_account(account.clone())?;

    // Record credit activity
    let activity = CreditActivity {
        timestamp: time(),
        principal_id: principal_id.clone(),
        amount,
        activity_type: CreditActivityType::Stack,
        status: TransferStatus::Completed,
        metadata: Some("Credit stacking".to_string()),
    };
    record_credit_activity(activity)?;

    Ok(result)
}

pub fn unstack_credits(principal_id: String, amount: u64) -> Result<AccountInfo, String> {
    let mut account = get_account(principal_id.clone())
        .ok_or_else(|| "Account not found".to_string())?;

    if account.get_staked_credits() < amount {
        return Err("Insufficient staked credits".to_string());
    }

    account.token_info.staked_credits -= amount;
    account.token_info.credit_balance += amount;
    account.updated_at = time();
    
    let result = upsert_account(account.clone())?;

    // Record credit activity
    let activity = CreditActivity {
        timestamp: time(),
        principal_id: principal_id.clone(),
        amount,
        activity_type: CreditActivityType::Unstack,
        status: TransferStatus::Completed,
        metadata: Some("Credit unstacking".to_string()),
    };
    record_credit_activity(activity)?;

    Ok(result)
}

// Token Operations
pub fn transfer_tokens(from: String, to: String, amount: u64) -> Result<AccountInfo, String> {
    let mut from_account = get_account(from.clone())
        .ok_or_else(|| "From account not found".to_string())?;
    
    let mut to_account = get_account(to.clone())
        .ok_or_else(|| "To account not found".to_string())?;
    
    if from_account.get_token_balance() < amount {
        return Err("Insufficient token balance".to_string());
    }

    from_account.token_info.token_balance -= amount;
    to_account.token_info.token_balance += amount;
    
    from_account.updated_at = time();
    to_account.updated_at = time();
    
    upsert_account(from_account.clone())?;
    upsert_account(to_account.clone())?;
    
    // Record token activity
    let activity = TokenActivity {
        timestamp: time(),
        from: from.clone(),
        to: to.clone(),
        amount,
        activity_type: TokenActivityType::Transfer,
        status: TransferStatus::Completed,
        metadata: Some("Token transfer".to_string()),
    };
    record_token_activity(activity)?;
    
    Ok(from_account)
}

// Credit Usage
pub fn use_credits(principal_id: String, amount: u64, service: String, metadata: Option<String>) -> Result<AccountInfo, String> {
    let mut account = get_account(principal_id.clone())
        .ok_or_else(|| "Account not found".to_string())?;
    
    if account.get_credit_balance() < amount {
        return Err("Insufficient credit balance".to_string());
    }

    account.token_info.credit_balance -= amount;
    account.updated_at = time();
    
    let result = upsert_account(account.clone())?;
    
    // Record credit activity
    let activity = CreditActivity {
        timestamp: time(),
        principal_id: principal_id.clone(),
        amount,
        activity_type: CreditActivityType::Spend,
        status: TransferStatus::Completed,
        metadata: Some(format!("Credit usage for service: {} - {}", service, metadata.unwrap_or_default())),
    };
    record_credit_activity(activity)?;
    
    Ok(result)
}

// Token Grant Operations
pub fn create_token_grant(grant: TokenGrant) -> Result<(), String> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        let key = TokenGrantKey {
            recipient: grant.recipient.clone(),
        };
        grants.borrow_mut().insert(key, grant);
        Ok(())
    })
}

pub fn claim_grant(principal_id: &str) -> Result<u64, String> {
    // First check if the account exists
    let account = get_account(principal_id.to_string())
        .ok_or_else(|| "Account not found".to_string())?;

    // Get the grant using the principal_id as recipient
    let grant = get_token_grant(principal_id)
        .ok_or_else(|| "No grant found for this account".to_string())?;

    let current_time = time();

    if current_time < grant.start_time {
        return Err("Grant period has not started".to_string());
    }

    let remaining_amount = grant.amount - grant.claimed_amount;
    if remaining_amount == 0 {
        return Err("No credits available to claim".to_string());
    }

    // Update grant
    let mut updated_grant = grant.clone();
    updated_grant.claimed_amount += remaining_amount;
    updated_grant.status = TokenGrantStatus::Active;
    create_token_grant(updated_grant)?;

    // Update account credit balance
    let mut account = account.clone();
    account.token_info.credit_balance += remaining_amount;
    account.updated_at = current_time;
    upsert_account(account)?;

    // Record credit activity
    let activity = CreditActivity {
        timestamp: current_time,
        principal_id: principal_id.to_string(),
        amount: remaining_amount,
        activity_type: CreditActivityType::Earn,
        status: TransferStatus::Completed,
        metadata: Some("Grant credit claim".to_string()),
    };
    record_credit_activity(activity)?;

    Ok(remaining_amount)
}

pub fn get_token_grant(recipient: &str) -> Option<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        let key = TokenGrantKey {
            recipient: recipient.to_string(),
        };
        grants.borrow().get(&key).map(|grant| grant.clone())
    })
}

pub fn get_all_token_grants() -> Vec<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        grants.borrow()
            .iter()
            .map(|(_, grant)| grant.clone())
            .collect()
    })
}

pub fn get_token_grants_paginated(offset: u64, limit: usize) -> Vec<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        grants.borrow()
            .iter()
            .skip(offset as usize)
            .take(limit)
            .map(|(_, grant)| grant.clone())
            .collect()
    })
}

pub fn get_token_grants_by_recipient(recipient: &str) -> Vec<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        grants.borrow()
            .iter()
            .filter(|(_, grant)| grant.recipient == recipient)
            .map(|(_, grant)| grant.clone())
            .collect()
    })
}

pub fn get_token_grants_by_status(status: &TokenGrantStatus) -> Vec<TokenGrant> {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        grants.borrow()
            .iter()
            .filter(|(_, grant)| grant.status == *status)
            .map(|(_, grant)| grant.clone())
            .collect()
    })
}

pub fn get_token_grants_count() -> u64 {
    crate::token_economy_types::TOKEN_GRANTS.with(|grants| {
        grants.borrow().len() as u64
    })
}

// Activity Recording
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

// Query Methods
pub fn get_account_token_info(principal_id: &str) -> Result<TokenInfo, String> {
    let account = get_account(principal_id.to_string())
        .ok_or_else(|| "Account not found".to_string())?;
    Ok(account.token_info)
}

pub fn get_balance_summary(principal_id: String) -> (u64, u64, u64, u64) {
    if let Some(account) = get_account(principal_id) {
        (
            account.get_token_balance(),
            account.get_staked_credits(),
            account.get_credit_balance(),
            0 // unclaimed_balance is not part of TokenInfo
        )
    } else {
        (0, 0, 0, 0)
    }
}

// Activity Query Methods
pub fn get_token_activities(principal_id: &str) -> Vec<TokenActivity> {
    crate::token_economy_types::TOKEN_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.from == principal_id || activity.to == principal_id)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

pub fn get_credit_activities(principal_id: &str) -> Vec<CreditActivity> {
    crate::token_economy_types::CREDIT_ACTIVITIES.with(|activities| {
        activities.borrow()
            .iter()
            .filter(|(_, activity)| activity.principal_id == principal_id)
            .map(|(_, activity)| activity.clone())
            .collect()
    })
}

// Activity Statistics
pub fn get_token_activity_statistics(principal_id: &str) -> (u64, u64, u64) {
    let activities = get_token_activities(principal_id);
    let total_count = activities.len() as u64;
    let total_amount = activities.iter().map(|a| a.amount).sum();
    let success_count = activities.iter()
        .filter(|a| a.status == TransferStatus::Completed)
        .count() as u64;
    (total_count, total_amount, success_count)
}

pub fn get_credit_activity_statistics(principal_id: &str) -> (u64, u64, u64) {
    let activities = get_credit_activities(principal_id);
    let total_count = activities.len() as u64;
    let total_amount = activities.iter().map(|a| a.amount).sum();
    let success_count = activities.iter()
        .filter(|a| a.status == TransferStatus::Completed)
        .count() as u64;
    (total_count, total_amount, success_count)
}

// Emission Policy Operations
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

pub fn get_emission_policy() -> Result<EmissionPolicy, String> {
    crate::token_economy_types::EMISSION_POLICY.with(|p| {
        p.borrow()
            .get(&"default".to_string())
            .ok_or_else(|| "Emission policy not found".to_string())
    })
}

pub fn update_emission_policy(policy: EmissionPolicy) -> Result<(), String> {
    crate::token_economy_types::EMISSION_POLICY.with(|p| {
        p.borrow_mut().insert("default".to_string(), policy);
        Ok(())
    })
}

// Activity Query Methods
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

// Credit Usage
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