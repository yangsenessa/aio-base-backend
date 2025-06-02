use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::stable_mem_storage::{AGENT_ITEMS, USER_AGENT_INDEX};

type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Both,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AgentItem {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub author: String,
    pub owner: String, // Principal ID as string
    pub platform: Option<Platform>,
    pub git_repo: String,
    pub homepage: Option<String>,
    pub input_params: Option<String>,
    pub output_example: Option<String>,
    pub image_url: Option<String>,
    pub exec_file_url: Option<String>,
    pub version: String
}

// Define the key for user data association
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserAgentKey {
    pub owner: String,
    pub item_id: u64,
}

impl ic_stable_structures::Storable for UserAgentKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner, &self.item_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, item_id) = Decode!(bytes.as_ref(), String, u64).unwrap();
        Self { owner, item_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for AgentItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    // Define a concrete bound instead of Unbounded
    const BOUND: Bound = Bound::Bounded { max_size: 2000 * 1024, is_fixed_size: false }; // 100KB should be sufficient
}

/// Add a new agent item to the storage
pub fn add_agent_item(mut agent: AgentItem) -> Result<u64, String> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow_mut(); // Removed mut from items
        let total_items = items.len();
        
        // Check if an agent with the same name already exists
        for i in 0..total_items {
            let existing = items.get(i).unwrap();
            if existing.name == agent.name {
                return Err(format!("Agent with name '{}' already exists", agent.name));
            }
        }
        
        // If name is unique, add the new agent
        let index = items.len();
        agent.id = index;
        items.push(&agent).unwrap();
        
        // Create owner index entry
        USER_AGENT_INDEX.with(|user_index| {
            let mut user_index = user_index.borrow_mut();
            let key = UserAgentKey { 
                owner: agent.owner.clone(), 
                item_id: index 
            };
            user_index.insert(key, ());
        });
        
        Ok(index)
    })
}

/// Get an agent item by index
pub fn get_agent_item(index: u64) -> Option<AgentItem> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow();
        if index < items.len() {
            Some(items.get(index).unwrap())
        } else {
            None
        }
    })
}

/// Get all agent items
pub fn get_all_agent_items() -> Vec<AgentItem> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow();
        let mut result = Vec::new();
        for i in 0..items.len() {
            result.push(items.get(i).unwrap());
        }
        result
    })
}

/// Get all agent items owned by a specific user
pub fn get_user_agent_items(owner: String) -> Vec<AgentItem> {
    let mut result = Vec::new();
    
    USER_AGENT_INDEX.with(|index| {
        let index = index.borrow();
        
        // Create range bounds for this user
        let start_key = UserAgentKey { owner: owner.clone(), item_id: 0 };
        let end_key = UserAgentKey { owner: owner.clone(), item_id: u64::MAX };
        
        // Get all items in range
        for (key, _) in index.range(start_key..=end_key) {
            if let Some(item) = get_agent_item(key.item_id) {
                result.push(item);
            }
        }
    });
    
    result
}

/// Update an existing agent item
pub fn update_agent_item(index: u64, mut agent: AgentItem) -> Result<(), String> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow_mut(); // Removed mut from items
        if index < items.len() {
            let existing = items.get(index).unwrap();
            
            // Check if the caller is the owner
            if existing.owner != agent.owner {
                return Err("Only the owner can update this item".to_string());
            }
            
            // Keep the ID and owner
            agent.id = index;
            
            items.set(index, &agent);
            Ok(())
        } else {
            Err("Index out of bounds".to_string())
        }
    })
}

/// Get agent items with pagination
pub fn get_agent_items_paginated(offset: u64, limit: usize) -> Vec<AgentItem> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow();
        let total_items = items.len();
        
        // If offset is beyond the end, return empty vec
        if offset >= total_items {
            return Vec::new();
        }
        
        // Calculate the end index
        let end = std::cmp::min(offset + limit as u64, total_items);
        
        // Collect the items in the range
        let mut result = Vec::new();
        for i in offset..end {
            result.push(items.get(i).unwrap());
        }
        
        result
    })
}


/// Get an agent item by name
pub fn get_agent_item_by_name(name: String) -> Option<AgentItem> {
    AGENT_ITEMS.with(|items| {
        let items = items.borrow();
        for i in 0..items.len() {
            let item = items.get(i).unwrap();
            if item.name == name {
                return Some(item);
            }
        }
        None
    })
}
