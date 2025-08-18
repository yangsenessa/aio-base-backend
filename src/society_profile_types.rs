use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{StableBTreeMap, StableVec};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use crate::stable_mem_storage::{USER_PROFILES, PRINCIPAL_INDEX, USER_ID_INDEX, EMAIL_INDEX};

// User profile data structure for society profile management
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub user_id: String,
    pub principal_id: String,
    pub name: Option<String>,           // Legacy compatibility
    pub nickname: String,
    pub login_method: LoginMethod,
    pub login_status: LoginStatus,
    pub email: Option<String>,
    pub picture: Option<String>,
    pub wallet_address: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: Option<String>,       // Additional metadata as JSON
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LoginMethod {
    Wallet,
    Google,
    II, // Internet Identity
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum LoginStatus {
    Authenticated,
    Unauthenticated,
}

// Define the key for user profile lookup by principal ID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrincipalKey {
    pub principal_id: String,
}

// Define the key for user profile lookup by user ID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserIdKey {
    pub user_id: String,
}

// Define the key for user profile lookup by email
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EmailKey {
    pub email: String,
}

impl ic_stable_structures::Storable for UserProfile {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 5 * 1024 * 1024, is_fixed_size: false }; // 5MB should be sufficient for user profiles
}

impl ic_stable_structures::Storable for PrincipalKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.principal_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let principal_id = Decode!(bytes.as_ref(), String).unwrap();
        Self { principal_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for UserIdKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.user_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let user_id = Decode!(bytes.as_ref(), String).unwrap();
        Self { user_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for EmailKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.email).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let email = Decode!(bytes.as_ref(), String).unwrap();
        Self { email }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

// Import storage from stable_mem_storage

/// Add or update a user profile
pub fn upsert_user_profile(profile: UserProfile) -> Result<u64, String> {
    let current_time = ic_cdk::api::time();
    let mut updated_profile = profile;
    updated_profile.updated_at = current_time;
    
    // Set created_at if it's a new profile
    if updated_profile.created_at == 0 {
        updated_profile.created_at = current_time;
    }
    
    USER_PROFILES.with(|profiles| {
        let mut profiles = profiles.borrow_mut();
        
        // Check if profile already exists by principal ID
        if let Some(existing_index) = PRINCIPAL_INDEX.with(|idx| idx.borrow().get(&PrincipalKey { principal_id: updated_profile.principal_id.clone() })) {
            // Update existing profile
            profiles.set(existing_index, &updated_profile);
            
            // Update indices
            update_indices(&updated_profile, existing_index)?;
            
            Ok(existing_index)
        } else {
            // Add new profile
            let index = profiles.len();
            profiles.push(&updated_profile)
                .map_err(|e| format!("Failed to store profile: {:?}", e))?;
            
            // Create indices
            create_indices(&updated_profile, index)?;
            
            Ok(index)
        }
    })
}

/// Get a user profile by principal ID
pub fn get_user_profile_by_principal(principal_id: String) -> Option<UserProfile> {
    PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        if let Some(profile_index) = index.get(&PrincipalKey { principal_id }) {
            get_user_profile(profile_index)
        } else {
            None
        }
    })
}

/// Get a user profile by user ID
pub fn get_user_profile_by_user_id(user_id: String) -> Option<UserProfile> {
    USER_ID_INDEX.with(|index| {
        let index = index.borrow();
        if let Some(profile_index) = index.get(&UserIdKey { user_id }) {
            get_user_profile(profile_index)
        } else {
            None
        }
    })
}

/// Get a user profile by email
pub fn get_user_profile_by_email(email: String) -> Option<UserProfile> {
    EMAIL_INDEX.with(|index| {
        let index = index.borrow();
        if let Some(profile_index) = index.get(&EmailKey { email }) {
            get_user_profile(profile_index)
        } else {
            None
        }
    })
}

/// Get a user profile by index
pub fn get_user_profile(index: u64) -> Option<UserProfile> {
    USER_PROFILES.with(|profiles| {
        let profiles = profiles.borrow();
        if index < profiles.len() {
            profiles.get(index)
        } else {
            None
        }
    })
}

/// Update user nickname
pub fn update_user_nickname(principal_id: String, nickname: String) -> Result<UserProfile, String> {
    if let Some(mut profile) = get_user_profile_by_principal(principal_id.clone()) {
        profile.nickname = nickname;
        profile.updated_at = ic_cdk::api::time();
        
        let index = upsert_user_profile(profile.clone())?;
        Ok(profile)
    } else {
        Err("User profile not found".to_string())
    }
}

/// Get all user profiles with pagination
pub fn get_user_profiles_paginated(offset: u64, limit: usize) -> Vec<UserProfile> {
    USER_PROFILES.with(|profiles| {
        let profiles = profiles.borrow();
        let total_profiles = profiles.len();
        
        if offset >= total_profiles {
            return Vec::new();
        }
        
        let end = std::cmp::min(offset + limit as u64, total_profiles);
        let mut result = Vec::new();
        
        for i in offset..end {
            if let Some(profile) = profiles.get(i) {
                result.push(profile);
            }
        }
        
        result
    })
}

/// Delete a user profile
pub fn delete_user_profile(principal_id: String) -> Result<bool, String> {
    PRINCIPAL_INDEX.with(|index| {
        if let Some(profile_index) = index.borrow().get(&PrincipalKey { principal_id: principal_id.clone() }) {
            // Remove from indices
            remove_indices(principal_id.clone())?;
            
            // Note: We don't actually remove from the main storage to maintain referential integrity
            // Instead, we mark it as deleted or keep it for audit purposes
            
            Ok(true)
        } else {
            Ok(false)
        }
    })
}

/// Get total number of user profiles
pub fn get_total_user_profiles() -> u64 {
    USER_PROFILES.with(|profiles| profiles.borrow().len())
}

// Helper functions for index management
fn create_indices(profile: &UserProfile, index: u64) -> Result<(), String> {
    // Create principal ID index
    PRINCIPAL_INDEX.with(|idx| {
        let mut idx = idx.borrow_mut();
        idx.insert(PrincipalKey { principal_id: profile.principal_id.clone() }, index);
    });
    
    // Create user ID index
    USER_ID_INDEX.with(|idx| {
        let mut idx = idx.borrow_mut();
        idx.insert(UserIdKey { user_id: profile.user_id.clone() }, index);
    });
    
    // Create email index if email exists
    if let Some(ref email) = profile.email {
        EMAIL_INDEX.with(|idx| {
            let mut idx = idx.borrow_mut();
            idx.insert(EmailKey { email: email.clone() }, index);
        });
    }
    
    Ok(())
}

fn update_indices(profile: &UserProfile, index: u64) -> Result<(), String> {
    // Remove old indices first
    remove_indices(profile.principal_id.clone())?;
    
    // Create new indices
    create_indices(profile, index)
}

fn remove_indices(principal_id: String) -> Result<(), String> {
    if let Some(profile) = get_user_profile_by_principal(principal_id.clone()) {
        // Remove from principal index
        PRINCIPAL_INDEX.with(|idx| {
            let mut idx = idx.borrow_mut();
            idx.remove(&PrincipalKey { principal_id: principal_id.clone() });
        });
        
        // Remove from user ID index
        USER_ID_INDEX.with(|idx| {
            let mut idx = idx.borrow_mut();
            idx.remove(&UserIdKey { user_id: profile.user_id });
        });
        
        // Remove from email index if email exists
        if let Some(ref email) = profile.email {
            EMAIL_INDEX.with(|idx| {
                let mut idx = idx.borrow_mut();
                idx.remove(&EmailKey { email: email.clone() });
            });
        }
    }
    
    Ok(())
}
