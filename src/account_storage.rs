use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::finance_types::AccountInfo;
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
        Decode!(bytes.as_ref(), Self).expect("Failed to decode AccountInfo")
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