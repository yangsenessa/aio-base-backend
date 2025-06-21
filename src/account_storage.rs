use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::token_economy_types::AccountInfo;
use std::collections::HashMap;
use candid::Principal;
use std::sync::LazyLock;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Define the key for account data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountKey {
    pub principal_id: String,
}

impl ic_stable_structures::Storable for AccountKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.principal_id).expect("Failed to encode AccountKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let principal_id = Decode!(bytes.as_ref(), String).expect("Failed to decode AccountKey");
        Self { principal_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for AccountInfo {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode AccountInfo"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        // Try to decode as current AccountInfo first
        if let Ok(account_info) = Decode!(bytes.as_ref(), Self) {
            return account_info;
        }
        
        // If that fails, try to decode as the old format and convert
        // The old format had flat fields instead of nested token_info
        if let Ok((principal_id, token_balance, credit_balance, staked_credits, kappa_multiplier, created_at, updated_at, metadata)) = 
            Decode!(bytes.as_ref(), (String, u64, u64, u64, f64, u64, Option<u64>, Option<String>)) {
            return Self {
                principal_id,
                token_info: crate::token_economy_types::TokenInfo {
                    token_balance,
                    credit_balance,
                    staked_credits,
                    kappa_multiplier,
                },
                created_at,
                updated_at,
                metadata,
            };
        }
        
        // If all decoding attempts fail, panic with a more descriptive error
        panic!("Failed to decode AccountInfo: data format is not compatible with current or previous versions");
    }
    const BOUND: Bound = Bound::Bounded { max_size: 20000 * 1024, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static ACCOUNTS: RefCell<StableBTreeMap<AccountKey, AccountInfo, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(10)))
        )
    );
}

/// Add or update an account
pub fn upsert_account(account: AccountInfo) -> Result<AccountInfo, String> {
    ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = AccountKey { principal_id: account.principal_id.clone() };
        
        // Insert will update if key exists, or insert if it doesn't
        accounts.insert(key, account.clone());
        Ok(account)
    })
}

/// Get an account by principal ID
pub fn get_account(principal_id: String) -> Option<AccountInfo> {
    ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        let key = AccountKey { principal_id };
        accounts.get(&key).map(|account| account.clone())
    })
}

/// Get all accounts
pub fn get_all_accounts() -> Vec<AccountInfo> {
    ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        accounts.iter().map(|(_, account)| account).collect()
    })
}

/// Delete an account
pub fn delete_account(principal_id: String) -> Result<(), String> {
    ACCOUNTS.with(|accounts| {
        let mut accounts = accounts.borrow_mut();
        let key = AccountKey { principal_id };
        if accounts.remove(&key).is_some() {
            Ok(())
        } else {
            Err("Account not found".to_string())
        }
    })
}

/// Get accounts with pagination
pub fn get_accounts_paginated(offset: u64, limit: usize) -> Vec<AccountInfo> {
    ACCOUNTS.with(|accounts| {
        let accounts = accounts.borrow();
        accounts
            .iter()
            .skip(offset as usize)
            .take(limit)
            .map(|(_, account)| account)
            .collect()
    })
} 