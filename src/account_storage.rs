use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::token_economy_types::{AccountInfo};
use crate::stable_mem_storage::ACCOUNTS;
use std::collections::HashMap;
use candid::Principal;
use std::sync::LazyLock;
use num_traits::ToPrimitive;

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
        ic_cdk::println!("Attempting to decode AccountInfo with {} bytes", bytes.len());
        
        // Try to decode as current AccountInfo first (with u64 fields)
        if let Ok(account_info) = Decode!(bytes.as_ref(), Self) {
            ic_cdk::println!("Successfully decoded as current AccountInfo format");
            return account_info;
        }
        
        // Try to decode as AccountInfo with candid::Nat fields (intermediate format)
        if let Ok((principal_id, token_balance, credit_balance, staked_credits, kappa_multiplier, created_at, updated_at, metadata)) = 
            Decode!(bytes.as_ref(), (String, candid::Nat, candid::Nat, candid::Nat, f64, u64, Option<u64>, Option<String>)) {
            ic_cdk::println!("Successfully decoded as candid::Nat format");
            return Self {
                principal_id,
                token_info: crate::token_economy_types::TokenInfo {
                    token_balance: token_balance.0.to_u64().unwrap_or(0),
                    credit_balance: credit_balance.0.to_u64().unwrap_or(0),
                    staked_credits: staked_credits.0.to_u64().unwrap_or(0),
                    kappa_multiplier,
                },
                created_at,
                updated_at,
                metadata,
            };
        }
        
        // Try to decode as the old format with flat fields (u64)
        if let Ok((principal_id, token_balance, credit_balance, staked_credits, kappa_multiplier, created_at, updated_at, metadata)) = 
            Decode!(bytes.as_ref(), (String, u64, u64, u64, f64, u64, Option<u64>, Option<String>)) {
            ic_cdk::println!("Successfully decoded as flat u64 format");
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
        
        // Try to decode as a simpler format with just principal_id and basic fields
        if let Ok((principal_id, token_balance, credit_balance, created_at)) = 
            Decode!(bytes.as_ref(), (String, u64, u64, u64)) {
            ic_cdk::println!("Successfully decoded as simple format");
            return Self {
                principal_id,
                token_info: crate::token_economy_types::TokenInfo {
                    token_balance,
                    credit_balance,
                    staked_credits: 0,
                    kappa_multiplier: 1.0,
                },
                created_at,
                updated_at: None,
                metadata: None,
            };
        }
        
        // Try to decode as a tuple with candid::Nat for timestamps
        if let Ok((principal_id, token_balance, credit_balance, staked_credits, kappa_multiplier, created_at, updated_at, metadata)) = 
            Decode!(bytes.as_ref(), (String, candid::Nat, candid::Nat, candid::Nat, f64, candid::Nat, Option<candid::Nat>, Option<String>)) {
            ic_cdk::println!("Successfully decoded as candid::Nat with candid::Nat timestamps");
            return Self {
                principal_id,
                token_info: crate::token_economy_types::TokenInfo {
                    token_balance: token_balance.0.to_u64().unwrap_or(0),
                    credit_balance: credit_balance.0.to_u64().unwrap_or(0),
                    staked_credits: staked_credits.0.to_u64().unwrap_or(0),
                    kappa_multiplier,
                },
                created_at: created_at.0.to_u64().unwrap_or(ic_cdk::api::time()),
                updated_at: updated_at.map(|t| t.0.to_u64().unwrap_or(0)),
                metadata,
            };
        }
        
        // If all decoding attempts fail, panic with detailed error information
        ic_cdk::println!("Error: Completely failed to decode AccountInfo data. Bytes length: {}", bytes.len());
        ic_cdk::println!("First 20 bytes: {:?}", &bytes[..std::cmp::min(20, bytes.len())]);
        
        panic!("Failed to decode AccountInfo: data format is not compatible with any known versions. Data may be corrupted. Bytes length: {}", bytes.len());
    }
    const BOUND: Bound = Bound::Bounded { max_size: 20000 * 1024, is_fixed_size: false };
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