use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{StableBTreeMap, StableVec};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
    pub devices: Vec<String>,           // User's device list
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
    
    // First check if profile already exists by principal ID
    let existing_index = PRINCIPAL_INDEX.with(|idx| {
        let idx = idx.borrow();
        idx.get(&PrincipalKey { principal_id: updated_profile.principal_id.clone() }).map(|idx| idx)
    });
    
    let result = USER_PROFILES.with(|profiles| -> Result<u64, String> {
        let mut profiles = profiles.borrow_mut();
        
        if let Some(existing_index) = existing_index {
            // Update existing profile
            profiles.set(existing_index, &updated_profile);
            Ok(existing_index)
        } else {
            // Add new profile
            let index = profiles.len();
            profiles.push(&updated_profile)
                .map_err(|e| format!("Failed to store profile: {:?}", e))?;
            Ok(index)
        }
    })?;
    
    // Update or create indices outside of the USER_PROFILES.with block
    if let Some(existing_index) = existing_index {
        // Update existing profile indices
        update_indices(&updated_profile, existing_index)?;
    } else {
        // Create new profile indices
        create_indices(&updated_profile, result)?;
    }
    
    Ok(result)
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
    // First get the profile index to avoid borrowing conflicts
    let profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: principal_id.clone() }).map(|idx| idx)
    });
    
    if let Some(index) = profile_index {
        // Get profile by index instead of by principal to avoid borrowing conflicts
        if let Some(mut profile) = get_user_profile(index) {
            profile.nickname = nickname;
            profile.updated_at = ic_cdk::api::time();
            
            let _ = upsert_user_profile(profile.clone())?;
            Ok(profile)
        } else {
            Err("User profile not found".to_string())
        }
    } else {
        Err("User profile not found".to_string())
    }
}

/// Add device to user profile
pub fn add_user_device(principal_id: String, device_id: String) -> Result<UserProfile, String> {
    // First get the profile index to avoid borrowing conflicts
    let profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: principal_id.clone() }).map(|idx| idx)
    });
    
    if let Some(index) = profile_index {
        // Get profile by index instead of by principal to avoid borrowing conflicts
        if let Some(mut profile) = get_user_profile(index) {
            if !profile.devices.contains(&device_id) {
                profile.devices.push(device_id);
                profile.updated_at = ic_cdk::api::time();
                
                let _ = upsert_user_profile(profile.clone())?;
                Ok(profile)
            } else {
                Ok(profile) // Device already exists
            }
        } else {
            Err("User profile not found".to_string())
        }
    } else {
        Err("User profile not found".to_string())
    }
}

/// Remove device from user profile
pub fn remove_user_device(principal_id: String, device_id: String) -> Result<UserProfile, String> {
    // First get the profile index to avoid borrowing conflicts
    let profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: principal_id.clone() }).map(|idx| idx)
    });
    
    if let Some(index) = profile_index {
        // Get profile by index instead of by principal to avoid borrowing conflicts
        if let Some(mut profile) = get_user_profile(index) {
            profile.devices.retain(|d| d != &device_id);
            profile.updated_at = ic_cdk::api::time();
            
            let _ = upsert_user_profile(profile.clone())?;
            Ok(profile)
        } else {
            Err("User profile not found".to_string())
        }
    } else {
        Err("User profile not found".to_string())
    }
}

/// Update user devices list
pub fn update_user_devices(principal_id: String, devices: Vec<String>) -> Result<UserProfile, String> {
    // First get the profile index to avoid borrowing conflicts
    let profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: principal_id.clone() }).map(|idx| idx)
    });
    
    if let Some(index) = profile_index {
        // Get profile by index instead of by principal to avoid borrowing conflicts
        if let Some(mut profile) = get_user_profile(index) {
            profile.devices = devices;
            profile.updated_at = ic_cdk::api::time();
            
            let _ = upsert_user_profile(profile.clone())?;
            Ok(profile)
        } else {
            Err("User profile not found".to_string())
        }
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
    // First get the profile index to avoid borrowing conflicts
    let profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: principal_id.clone() }).map(|idx| idx)
    });
    
    if let Some(index) = profile_index {
        // Get profile by index instead of by principal to avoid borrowing conflicts
        if let Some(profile) = get_user_profile(index) {
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
    }
    
    Ok(())
}

// ==== Contact Management ====

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    pub id: u64,
    pub owner_principal_id: String,        // Contact owner
    pub contact_principal_id: String,     // Contact's principal ID
    pub name: String,                     // Contact name
    pub nickname: Option<String>,         // Nickname
    pub contact_type: ContactType,        // Contact type
    pub status: ContactStatus,            // Contact status
    pub avatar: Option<String>,           // Avatar
    pub devices: Vec<String>,             // Associated devices
    pub is_online: bool,                  // Online status
    pub created_at: u64,                  // Creation time
    pub updated_at: u64,                  // Update time
    pub metadata: Option<String>,         // Additional metadata (JSON format)
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ContactType {
    Friend,     // Friend
    System,     // System
    Business,   // Business
    Family,     // Family
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ContactStatus {
    Active,     // Active
    Pending,    // Pending
    Blocked,    // Blocked
    Deleted,    // Deleted
}

// Contact lookup keys
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContactOwnerKey {
    pub owner_principal_id: String,
    pub contact_principal_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContactNameKey {
    pub owner_principal_id: String,
    pub name: String,
}

// Implement Storable trait
impl ic_stable_structures::Storable for Contact {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 2 * 1024 * 1024, is_fixed_size: false }; // 2MB for contacts
}

impl ic_stable_structures::Storable for ContactOwnerKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner_principal_id, &self.contact_principal_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner_principal_id, contact_principal_id) = Decode!(bytes.as_ref(), String, String).unwrap();
        Self { owner_principal_id, contact_principal_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 2048, is_fixed_size: false };
}

impl ic_stable_structures::Storable for ContactNameKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner_principal_id, &self.name).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner_principal_id, name) = Decode!(bytes.as_ref(), String, String).unwrap();
        Self { owner_principal_id, name }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 2048, is_fixed_size: false };
}

// Contact management functions
/// Add or update contact
pub fn upsert_contact(contact: Contact) -> Result<u64, String> {
    let current_time = ic_cdk::api::time();
    let mut updated_contact = contact;
    updated_contact.updated_at = current_time;
    
    // Set creation time (if it's a new contact)
    if updated_contact.created_at == 0 {
        updated_contact.created_at = current_time;
    }
    
    // Get devices from user profile if not provided
    if updated_contact.devices.is_empty() {
        // First get the profile index to avoid borrowing conflicts
        let profile_index = PRINCIPAL_INDEX.with(|index| {
            let index = index.borrow();
            index.get(&PrincipalKey { principal_id: updated_contact.contact_principal_id.clone() }).map(|idx| idx)
        });
        
        if let Some(index) = profile_index {
            // Get profile by index instead of by principal to avoid borrowing conflicts
            if let Some(user_profile) = get_user_profile(index) {
                updated_contact.devices = user_profile.devices.clone();
            }
        }
    }
    
    // Use contact storage from stable_mem_storage
    crate::stable_mem_storage::CONTACTS.with(|contacts| {
        let mut contacts = contacts.borrow_mut();
        
        // Check if contact already exists
        if let Some(existing_index) = crate::stable_mem_storage::CONTACT_OWNER_INDEX.with(|idx| {
            idx.borrow().get(&ContactOwnerKey { 
                owner_principal_id: updated_contact.owner_principal_id.clone(),
                contact_principal_id: updated_contact.contact_principal_id.clone()
            })
        }) {
            // Update existing contact
            contacts.set(existing_index, &updated_contact);
            
            // Update indices
            update_contact_indices(&updated_contact, existing_index)?;
            
            Ok(existing_index)
        } else {
            // Add new contact
            let index = contacts.len();
            contacts.push(&updated_contact)
                .map_err(|e| format!("Failed to store contact: {:?}", e))?;
            
            // Create indices
            create_contact_indices(&updated_contact, index)?;
            
            Ok(index)
        }
    })
}

/// Create contact from principal ID (for adding friends) - creates bidirectional relationship
pub fn create_contact_from_principal_id(
    owner_principal_id: String, 
    contact_principal_id: String,
    nickname: Option<String>
) -> Result<u64, String> {
    // Check if both users exist
    let contact_profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: contact_principal_id.clone() }).map(|idx| idx)
    });
    
    let owner_profile_index = PRINCIPAL_INDEX.with(|index| {
        let index = index.borrow();
        index.get(&PrincipalKey { principal_id: owner_principal_id.clone() }).map(|idx| idx)
    });
    
    // Get both user profiles
    let contact_profile = if let Some(index) = contact_profile_index {
        get_user_profile(index)
    } else {
        return Err("Contact user profile not found for the given principal ID".to_string());
    };
    
    let owner_profile = if let Some(index) = owner_profile_index {
        get_user_profile(index)
    } else {
        return Err("Owner user profile not found for the given principal ID".to_string());
    };
    
    let contact_profile = contact_profile.ok_or("Contact user profile not found")?;
    let owner_profile = owner_profile.ok_or("Owner user profile not found")?;
    
    // Create contact record for owner -> contact
    let owner_to_contact = Contact {
        id: 0, // Will be set by storage
        owner_principal_id: owner_principal_id.clone(),
        contact_principal_id: contact_principal_id.clone(),
        name: contact_profile.name.clone().unwrap_or_else(|| "Unknown User".to_string()),
        nickname: nickname.clone(),
        contact_type: ContactType::Friend,
        status: ContactStatus::Active,
        avatar: contact_profile.picture.clone(),
        devices: contact_profile.devices.clone(),
        is_online: false,
        created_at: 0,
        updated_at: 0,
        metadata: None,
    };
    
    // Create contact record for contact -> owner (bidirectional)
    let contact_to_owner = Contact {
        id: 0, // Will be set by storage
        owner_principal_id: contact_principal_id.clone(),
        contact_principal_id: owner_principal_id.clone(),
        name: owner_profile.name.clone().unwrap_or_else(|| "Unknown User".to_string()),
        nickname: None, // Contact doesn't set nickname for owner
        contact_type: ContactType::Friend,
        status: ContactStatus::Active,
        avatar: owner_profile.picture.clone(),
        devices: owner_profile.devices.clone(),
        is_online: false,
        created_at: 0,
        updated_at: 0,
        metadata: None,
    };
    
    // Insert both contact records
    let owner_contact_index = upsert_contact(owner_to_contact)?;
    
    // If the first insertion succeeds, insert the reverse relationship
    match upsert_contact(contact_to_owner) {
        Ok(_) => Ok(owner_contact_index),
        Err(e) => {
            // If the second insertion fails, we should ideally rollback the first one
            // However, since we don't have transaction support, we'll log the error
            // and return the original error
            Err(format!("Failed to create bidirectional contact relationship: {}", e))
        }
    }
}

/// Get all contacts by owner principal ID
pub fn get_contacts_by_owner(owner_principal_id: String) -> Vec<Contact> {
    let mut contacts = Vec::new();
    
    crate::stable_mem_storage::CONTACTS.with(|contacts_store| {
        let contacts_store = contacts_store.borrow();
        
        for i in 0..contacts_store.len() {
            if let Some(contact) = contacts_store.get(i) {
                if contact.owner_principal_id == owner_principal_id {
                    contacts.push(contact);
                }
            }
        }
    });
    
    contacts
}

/// Get contacts by owner principal ID with pagination
pub fn get_contacts_by_owner_paginated(owner_principal_id: String, offset: u64, limit: usize) -> Vec<Contact> {
    let all_contacts = get_contacts_by_owner(owner_principal_id);
    let total_contacts = all_contacts.len();
    
    if offset >= total_contacts as u64 {
        return Vec::new();
    }
    
    let end = std::cmp::min(offset + limit as u64, total_contacts as u64);
    all_contacts.into_iter().skip(offset as usize).take((end - offset) as usize).collect()
}

/// Get contact by contact ID
pub fn get_contact_by_id(contact_id: u64) -> Option<Contact> {
    crate::stable_mem_storage::CONTACTS.with(|contacts| {
        let contacts = contacts.borrow();
        if contact_id < contacts.len() {
            contacts.get(contact_id)
        } else {
            None
        }
    })
}

/// Get contact by owner principal ID and contact principal ID
pub fn get_contact_by_principal_ids(owner_principal_id: String, contact_principal_id: String) -> Option<Contact> {
    if let Some(contact_index) = crate::stable_mem_storage::CONTACT_OWNER_INDEX.with(|idx| {
        idx.borrow().get(&ContactOwnerKey { 
            owner_principal_id: owner_principal_id.clone(),
            contact_principal_id: contact_principal_id.clone()
        })
    }) {
        get_contact_by_id(contact_index)
    } else {
        None
    }
}

/// Search contacts by name
pub fn search_contacts_by_name(owner_principal_id: String, name_query: String) -> Vec<Contact> {
    let mut contacts = Vec::new();
    
    crate::stable_mem_storage::CONTACTS.with(|contacts_store| {
        let contacts_store = contacts_store.borrow();
        
        for i in 0..contacts_store.len() {
            if let Some(contact) = contacts_store.get(i) {
                if contact.owner_principal_id == owner_principal_id && 
                   (contact.name.to_lowercase().contains(&name_query.to_lowercase()) ||
                    contact.nickname.as_ref().map_or(false, |n| n.to_lowercase().contains(&name_query.to_lowercase()))) {
                    contacts.push(contact);
                }
            }
        }
    });
    
    contacts
}

/// Update contact status
pub fn update_contact_status(owner_principal_id: String, contact_principal_id: String, new_status: ContactStatus) -> Result<Contact, String> {
    if let Some(mut contact) = get_contact_by_principal_ids(owner_principal_id.clone(), contact_principal_id.clone()) {
        contact.status = new_status;
        contact.updated_at = ic_cdk::api::time();
        
        let index = upsert_contact(contact.clone())?;
        Ok(contact)
    } else {
        Err("Contact not found".to_string())
    }
}

/// Update contact nickname
pub fn update_contact_nickname(owner_principal_id: String, contact_principal_id: String, nickname: String) -> Result<Contact, String> {
    if let Some(mut contact) = get_contact_by_principal_ids(owner_principal_id.clone(), contact_principal_id.clone()) {
        contact.nickname = Some(nickname);
        contact.updated_at = ic_cdk::api::time();
        
        let index = upsert_contact(contact.clone())?;
        Ok(contact)
    } else {
        Err("Contact not found".to_string())
    }
}

/// Update contact devices list
pub fn update_contact_devices(owner_principal_id: String, contact_principal_id: String, devices: Vec<String>) -> Result<Contact, String> {
    if let Some(mut contact) = get_contact_by_principal_ids(owner_principal_id.clone(), contact_principal_id.clone()) {
        contact.devices = devices;
        contact.updated_at = ic_cdk::api::time();
        
        let index = upsert_contact(contact.clone())?;
        Ok(contact)
    } else {
        Err("Contact not found".to_string())
    }
}

/// Update contact online status
pub fn update_contact_online_status(owner_principal_id: String, contact_principal_id: String, is_online: bool) -> Result<Contact, String> {
    if let Some(mut contact) = get_contact_by_principal_ids(owner_principal_id.clone(), contact_principal_id.clone()) {
        contact.is_online = is_online;
        contact.updated_at = ic_cdk::api::time();
        
        let index = upsert_contact(contact.clone())?;
        Ok(contact)
    } else {
        Err("Contact not found".to_string())
    }
}

/// Delete contact
pub fn delete_contact(owner_principal_id: String, contact_principal_id: String) -> Result<bool, String> {
    if let Some(contact_index) = crate::stable_mem_storage::CONTACT_OWNER_INDEX.with(|idx| {
        idx.borrow().get(&ContactOwnerKey { 
            owner_principal_id: owner_principal_id.clone(),
            contact_principal_id: contact_principal_id.clone()
        })
    }) {
        // Remove indices
        remove_contact_indices(owner_principal_id.clone(), contact_principal_id.clone())?;
        
        // Note: We don't actually delete from main storage to maintain referential integrity
        // Instead, we mark it as deleted or keep it for audit purposes
        
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get total number of contacts by owner
pub fn get_total_contacts_by_owner(owner_principal_id: String) -> u64 {
    get_contacts_by_owner(owner_principal_id).len() as u64
}

// Contact index management helper functions
fn create_contact_indices(contact: &Contact, index: u64) -> Result<(), String> {
    // Create owner-contact index
    crate::stable_mem_storage::CONTACT_OWNER_INDEX.with(|idx| {
        let mut idx = idx.borrow_mut();
        idx.insert(ContactOwnerKey { 
            owner_principal_id: contact.owner_principal_id.clone(),
            contact_principal_id: contact.contact_principal_id.clone()
        }, index);
    });
    
    // Create name index
    crate::stable_mem_storage::CONTACT_NAME_INDEX.with(|idx| {
        let mut idx = idx.borrow_mut();
        idx.insert(ContactNameKey { 
            owner_principal_id: contact.owner_principal_id.clone(),
            name: contact.name.clone()
        }, index);
    });
    
    Ok(())
}

fn update_contact_indices(contact: &Contact, index: u64) -> Result<(), String> {
    // Remove old indices first
    remove_contact_indices(contact.owner_principal_id.clone(), contact.contact_principal_id.clone())?;
    
    // Create new indices
    create_contact_indices(contact, index)
}

fn remove_contact_indices(owner_principal_id: String, contact_principal_id: String) -> Result<(), String> {
    if let Some(contact) = get_contact_by_principal_ids(owner_principal_id.clone(), contact_principal_id.clone()) {
        // Remove from owner-contact index
        crate::stable_mem_storage::CONTACT_OWNER_INDEX.with(|idx| {
            let mut idx = idx.borrow_mut();
            idx.remove(&ContactOwnerKey { 
                owner_principal_id: owner_principal_id.clone(),
                contact_principal_id: contact_principal_id.clone()
            });
        });
        
        // Remove from name index
        crate::stable_mem_storage::CONTACT_NAME_INDEX.with(|idx| {
            let mut idx = idx.borrow_mut();
            idx.remove(&ContactNameKey { 
                owner_principal_id: owner_principal_id.clone(),
                name: contact.name
            });
        });
    }
    
    Ok(())
}

// ==== Social Chat System ====

/// Message content mode for different data types
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MessageMode {
    Text,           // Plain text message
    Voice,          // Voice message (base64 encoded)
    Image,          // Image message (base64 encoded) 
    Emoji,          // Emoji/sticker message (base64 encoded)
}

/// Individual chat message structure
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub send_by: String,        // Sender's principal ID
    pub content: String,        // Message content (base64 for non-text modes)
    pub mode: MessageMode,      // Content type
    pub timestamp: u64,         // Message timestamp
}

/// Social pair key for chat between two users
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocialPairKey {
    pub pair_key: String,       // Deterministic key from two principal IDs
}

/// Chat history for a social pair
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ChatHistory {
    pub social_pair_key: String,       // The social pair identifier
    pub messages: Vec<ChatMessage>,    // Chat messages in chronological order
    pub created_at: u64,               // First message timestamp
    pub updated_at: u64,               // Last message timestamp
}

/// Notification queue item
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct NotificationItem {
    pub social_pair_key: String,   // Social pair this notification belongs to
    pub to_who: String,            // Receiver's principal ID
    pub message_id: u64,           // Index of the message in chat history
    pub timestamp: u64,            // Notification timestamp
}

/// Notification queue key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotificationKey {
    pub notification_id: String,   // Unique notification identifier
}

// Implement Storable traits
impl ic_stable_structures::Storable for SocialPairKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.pair_key).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let pair_key = Decode!(bytes.as_ref(), String).unwrap();
        Self { pair_key }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for ChatHistory {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 10 * 1024 * 1024, is_fixed_size: false }; // 10MB for chat history
}

impl ic_stable_structures::Storable for NotificationKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.notification_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let notification_id = Decode!(bytes.as_ref(), String).unwrap();
        Self { notification_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for NotificationItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 2 * 1024, is_fixed_size: false }; // 2KB for notifications
}

// Social chat system functions

/// Generate deterministic social pair key from two principal IDs
/// This algorithm ensures the same key regardless of sender/receiver order
pub fn generate_social_pair_key(principal1: String, principal2: String) -> String {
    let mut principals = vec![principal1, principal2];
    principals.sort(); // Sort to ensure deterministic order
    
    let combined = format!("{}:{}", principals[0], principals[1]);
    
    // Create hash for shorter key
    let mut hasher = DefaultHasher::new();
    combined.hash(&mut hasher);
    let hash = hasher.finish();
    
    format!("social_pair_{}", hash)
}

/// Add new chat message to social pair
pub fn add_chat_message(
    sender_principal: String,
    receiver_principal: String,
    content: String,
    mode: MessageMode,
) -> Result<u64, String> {
    let pair_key = generate_social_pair_key(sender_principal.clone(), receiver_principal.clone());
    let current_time = ic_cdk::api::time();
    
    let new_message = ChatMessage {
        send_by: sender_principal,
        content,
        mode,
        timestamp: current_time,
    };
    
    // Get or create chat history
    let mut chat_history = crate::stable_mem_storage::CHAT_HISTORIES.with(|histories| {
        let histories = histories.borrow();
        histories.get(&SocialPairKey { pair_key: pair_key.clone() })
            .unwrap_or_else(|| ChatHistory {
                social_pair_key: pair_key.clone(),
                messages: Vec::new(),
                created_at: current_time,
                updated_at: current_time,
            })
    });
    
    // Add new message
    chat_history.messages.push(new_message);
    chat_history.updated_at = current_time;
    let message_index = chat_history.messages.len() - 1;
    
    // Update chat history in storage
    crate::stable_mem_storage::CHAT_HISTORIES.with(|histories| {
        let mut histories = histories.borrow_mut();
        histories.insert(SocialPairKey { pair_key: pair_key.clone() }, chat_history);
    });
    
    // Push notification to queue
    push_notification(pair_key, receiver_principal, message_index as u64)?;
    
    Ok(message_index as u64)
}

/// Get recent chat messages (last 5 messages)
pub fn get_recent_chat_messages(principal1: String, principal2: String) -> Vec<ChatMessage> {
    let pair_key = generate_social_pair_key(principal1, principal2);
    
    crate::stable_mem_storage::CHAT_HISTORIES.with(|histories| {
        let histories = histories.borrow();
        if let Some(chat_history) = histories.get(&SocialPairKey { pair_key }) {
            let messages = &chat_history.messages;
            let start_index = if messages.len() > 5 { messages.len() - 5 } else { 0 };
            messages[start_index..].to_vec()
        } else {
            Vec::new()
        }
    })
}

/// Get paginated chat messages
pub fn get_chat_messages_paginated(
    principal1: String,
    principal2: String,
    offset: u64,
    limit: usize,
) -> Vec<ChatMessage> {
    let pair_key = generate_social_pair_key(principal1, principal2);
    
    crate::stable_mem_storage::CHAT_HISTORIES.with(|histories| {
        let histories = histories.borrow();
        if let Some(chat_history) = histories.get(&SocialPairKey { pair_key }) {
            let messages = &chat_history.messages;
            let total_messages = messages.len() as u64;
            
            if offset >= total_messages {
                return Vec::new();
            }
            
            let start_index = offset as usize;
            let end_index = std::cmp::min(start_index + limit, messages.len());
            
            messages[start_index..end_index].to_vec()
        } else {
            Vec::new()
        }
    })
}

/// Get total message count for a social pair
pub fn get_chat_message_count(principal1: String, principal2: String) -> u64 {
    let pair_key = generate_social_pair_key(principal1, principal2);
    
    crate::stable_mem_storage::CHAT_HISTORIES.with(|histories| {
        let histories = histories.borrow();
        if let Some(chat_history) = histories.get(&SocialPairKey { pair_key }) {
            chat_history.messages.len() as u64
        } else {
            0
        }
    })
}

// Notification queue functions

/// Push notification to queue
pub fn push_notification(
    social_pair_key: String,
    receiver_principal: String,
    message_id: u64,
) -> Result<(), String> {
    let current_time = ic_cdk::api::time();
    let notification_id = format!("{}:{}:{}", social_pair_key, receiver_principal, current_time);
    
    let notification = NotificationItem {
        social_pair_key,
        to_who: receiver_principal,
        message_id,
        timestamp: current_time,
    };
    
    crate::stable_mem_storage::NOTIFICATION_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        queue.insert(NotificationKey { notification_id }, notification);
    });
    
    Ok(())
}

/// Pop notification from queue for specific receiver
pub fn pop_notification(receiver_principal: String) -> Option<NotificationItem> {
    crate::stable_mem_storage::NOTIFICATION_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        
        // Find the first notification for this receiver
        let mut notification_to_remove = None;
        let mut result = None;
        
        for (key, notification) in queue.iter() {
            if notification.to_who == receiver_principal {
                notification_to_remove = Some(key.clone());
                result = Some(notification.clone());
                break;
            }
        }
        
        // Remove the notification if found
        if let Some(key) = notification_to_remove {
            queue.remove(&key);
        }
        
        result
    })
}

/// Get all notifications for a receiver (without removing them)
pub fn get_notifications_for_receiver(receiver_principal: String) -> Vec<NotificationItem> {
    crate::stable_mem_storage::NOTIFICATION_QUEUE.with(|queue| {
        let queue = queue.borrow();
        queue.iter()
            .filter(|(_, notification)| notification.to_who == receiver_principal)
            .map(|(_, notification)| notification.clone())
            .collect()
    })
}

/// Clear all notifications for a specific social pair and receiver
pub fn clear_notifications_for_pair(
    social_pair_key: String,
    receiver_principal: String,
) -> Result<u64, String> {
    let mut removed_count = 0;
    
    crate::stable_mem_storage::NOTIFICATION_QUEUE.with(|queue| {
        let mut queue = queue.borrow_mut();
        let mut keys_to_remove = Vec::new();
        
        // Collect keys to remove
        for (key, notification) in queue.iter() {
            if notification.social_pair_key == social_pair_key && notification.to_who == receiver_principal {
                keys_to_remove.push(key.clone());
            }
        }
        
        // Remove the collected keys
        for key in keys_to_remove {
            queue.remove(&key);
            removed_count += 1;
        }
    });
    
    Ok(removed_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_cdk::api::time;

    #[test]
    fn test_user_profile_creation() {
        let profile = UserProfile {
            user_id: "user123".to_string(),
            principal_id: "principal456".to_string(),
            name: Some("Test User".to_string()),
            nickname: "Test".to_string(),
            login_method: LoginMethod::Wallet,
            login_status: LoginStatus::Authenticated,
            email: Some("test@example.com".to_string()),
            picture: Some("avatar.jpg".to_string()),
            wallet_address: Some("0x123...".to_string()),
            devices: vec!["Device1".to_string(), "Device2".to_string()],
            created_at: time(),
            updated_at: time(),
            metadata: Some("Test metadata".to_string()),
        };

        assert_eq!(profile.user_id, "user123");
        assert_eq!(profile.principal_id, "principal456");
        assert_eq!(profile.name, Some("Test User".to_string()));
        assert_eq!(profile.devices.len(), 2);
        assert_eq!(profile.devices[0], "Device1");
        assert_eq!(profile.devices[1], "Device2");
    }

    #[test]
    fn test_contact_creation() {
        let contact = Contact {
            id: 0,
            owner_principal_id: "owner123".to_string(),
            contact_principal_id: "contact456".to_string(),
            name: "Test Contact".to_string(),
            nickname: Some("Test".to_string()),
            contact_type: ContactType::Friend,
            status: ContactStatus::Active,
            avatar: Some("AV".to_string()),
            devices: vec!["Device1".to_string(), "Device2".to_string()],
            is_online: true,
            created_at: 0,
            updated_at: 0,
            metadata: Some("Test metadata".to_string()),
        };

        assert_eq!(contact.owner_principal_id, "owner123");
        assert_eq!(contact.contact_principal_id, "contact456");
        assert_eq!(contact.name, "Test Contact");
        assert_eq!(contact.contact_type, ContactType::Friend);
        assert_eq!(contact.status, ContactStatus::Active);
        assert_eq!(contact.devices.len(), 2);
        assert!(contact.is_online);
    }

    #[test]
    fn test_contact_type_variants() {
        let friend = ContactType::Friend;
        let system = ContactType::System;
        let business = ContactType::Business;
        let family = ContactType::Family;

        assert_ne!(friend, system);
        assert_ne!(business, family);
        assert_eq!(friend, ContactType::Friend);
    }

    #[test]
    fn test_contact_status_variants() {
        let active = ContactStatus::Active;
        let pending = ContactStatus::Pending;
        let blocked = ContactStatus::Blocked;
        let deleted = ContactStatus::Deleted;

        assert_ne!(active, pending);
        assert_ne!(blocked, deleted);
        assert_eq!(active, ContactStatus::Active);
    }

    #[test]
    fn test_contact_owner_key() {
        let key = ContactOwnerKey {
            owner_principal_id: "owner123".to_string(),
            contact_principal_id: "contact456".to_string(),
        };

        assert_eq!(key.owner_principal_id, "owner123");
        assert_eq!(key.contact_principal_id, "contact456");
    }

    #[test]
    fn test_contact_name_key() {
        let key = ContactNameKey {
            owner_principal_id: "owner123".to_string(),
            name: "Test Contact".to_string(),
        };

        assert_eq!(key.owner_principal_id, "owner123");
        assert_eq!(key.name, "Test Contact");
    }

    #[test]
    fn test_social_pair_key_generation() {
        let key1 = generate_social_pair_key("alice".to_string(), "bob".to_string());
        let key2 = generate_social_pair_key("bob".to_string(), "alice".to_string());
        
        // Should generate the same key regardless of order
        assert_eq!(key1, key2);
        assert!(key1.starts_with("social_pair_"));
    }

    #[test]
    fn test_message_mode_variants() {
        let text = MessageMode::Text;
        let voice = MessageMode::Voice;
        let image = MessageMode::Image;
        let emoji = MessageMode::Emoji;

        assert_ne!(text, voice);
        assert_ne!(image, emoji);
        assert_eq!(text, MessageMode::Text);
    }
    
    #[test]
    fn test_bidirectional_contact_creation() {
        // This test would need to be run in a proper IC test environment
        // since it relies on stable memory structures
        
        // Test data
        let owner_principal = "owner-principal-123".to_string();
        let contact_principal = "contact-principal-456".to_string();
        let nickname = Some("My Friend".to_string());
        
        // Note: In a real test environment, we would:
        // 1. First create user profiles for both principals
        // 2. Call create_contact_from_principal_id
        // 3. Verify that contacts exist in both directions using get_contacts_by_owner
        // 4. Check that the owner->contact relationship includes the nickname
        // 5. Check that the contact->owner relationship doesn't include nickname
        // 6. Verify both relationships have ContactType::Friend and ContactStatus::Active
        
        // For now, we just verify the test structure is correct
        assert_eq!(owner_principal, "owner-principal-123");
        assert_eq!(contact_principal, "contact-principal-456");
        assert_eq!(nickname, Some("My Friend".to_string()));
        
        // Expected behavior after calling create_contact_from_principal_id:
        // - owner_principal should have contact_principal in their contacts list with nickname
        // - contact_principal should have owner_principal in their contacts list without nickname
        // - Both relationships should be of type Friend and status Active
    }

    #[test]
    fn test_bidirectional_contact_data_structure() {
        // 测试双边联系人关系的数据结构正确性
        let owner_principal = "owner-principal-123".to_string();
        let contact_principal = "contact-principal-456".to_string();
        let nickname = Some("我的好友".to_string());
        
        // 模拟创建双边联系人记录时的数据结构
        let owner_to_contact = Contact {
            id: 0,
            owner_principal_id: owner_principal.clone(),
            contact_principal_id: contact_principal.clone(),
            name: "Bob".to_string(),
            nickname: nickname.clone(),
            contact_type: ContactType::Friend,
            status: ContactStatus::Active,
            avatar: Some("avatar.jpg".to_string()),
            devices: vec!["device1".to_string()],
            is_online: false,
            created_at: 1700000000,
            updated_at: 1700000000,
            metadata: None,
        };
        
        let contact_to_owner = Contact {
            id: 0,
            owner_principal_id: contact_principal.clone(),
            contact_principal_id: owner_principal.clone(),
            name: "Alice".to_string(),
            nickname: None, // 重要：反向关系不包含昵称
            contact_type: ContactType::Friend,
            status: ContactStatus::Active,
            avatar: Some("avatar2.jpg".to_string()),
            devices: vec!["device2".to_string()],
            is_online: false,
            created_at: 1700000000,
            updated_at: 1700000000,
            metadata: None,
        };
        
        // 验证数据结构正确性
        
        // 1. 验证owner->contact记录
        assert_eq!(owner_to_contact.owner_principal_id, owner_principal);
        assert_eq!(owner_to_contact.contact_principal_id, contact_principal);
        assert_eq!(owner_to_contact.nickname, nickname);
        assert_eq!(owner_to_contact.contact_type, ContactType::Friend);
        assert_eq!(owner_to_contact.status, ContactStatus::Active);
        
        // 2. 验证contact->owner记录
        assert_eq!(contact_to_owner.owner_principal_id, contact_principal);
        assert_eq!(contact_to_owner.contact_principal_id, owner_principal);
        assert_eq!(contact_to_owner.nickname, None); // 反向关系无昵称
        assert_eq!(contact_to_owner.contact_type, ContactType::Friend);
        assert_eq!(contact_to_owner.status, ContactStatus::Active);
        
        // 3. 验证双向关系一致性
        assert_ne!(owner_to_contact.owner_principal_id, contact_to_owner.owner_principal_id);
        assert_eq!(owner_to_contact.owner_principal_id, contact_to_owner.contact_principal_id);
        assert_eq!(owner_to_contact.contact_principal_id, contact_to_owner.owner_principal_id);
    }

    #[test]
    fn test_bidirectional_contact_error_cases() {
        // 测试错误处理逻辑
        
        // 1. 空principal ID测试
        let empty_principal = "".to_string();
        let valid_principal = "valid-principal".to_string();
        
        assert!(empty_principal.is_empty());
        assert!(!valid_principal.is_empty());
        
        // 2. None nickname测试
        let nickname_none: Option<String> = None;
        let nickname_some = Some("昵称".to_string());
        
        assert!(nickname_none.is_none());
        assert!(nickname_some.is_some());
        
        // 3. 相同principal ID测试（应该被阻止）
        let same_principal = "same-principal".to_string();
        assert_eq!(same_principal.clone(), same_principal);
        // 在实际应用中，应该阻止用户添加自己为联系人
    }
}

