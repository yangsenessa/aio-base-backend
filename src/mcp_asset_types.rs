use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct McpItem {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub author: String,
    pub owner: String, // Principal ID as string
    pub git_repo: String,
    pub exec_file:Option<String>,
    pub homepage: Option<String>,
    pub remote_endpoint: Option<String>,
    pub mcp_type: String, // 'stdio' | 'http' | 'sse'
    pub community_body: Option<String>,
    // MCP capabilities
    pub resources: bool,
    pub prompts: bool,
    pub tools: bool,
    pub sampling: bool,
}

// Define the key for user data association
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserMcpKey {
    pub owner: String,
    pub item_id: u64,
}

impl ic_stable_structures::Storable for UserMcpKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner, &self.item_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, item_id) = Decode!(bytes.as_ref(), String, u64).unwrap();
        Self { owner, item_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for McpItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    // Define a concrete bound instead of Unbounded
    const BOUND: Bound = Bound::Bounded { max_size: 2000 * 1024, is_fixed_size: false }; // 100KB should be sufficient
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static MCP_ITEMS: RefCell<StableVec<McpItem, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ).unwrap()
    );
    
    static USER_MCP_INDEX: RefCell<StableBTreeMap<UserMcpKey, (), Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
        )
    );
}

/// Add a new MCP item to the storage
pub fn add_mcp_item( mcp: McpItem, caller_id: String) -> Result<u64, String> {
    MCP_ITEMS.with(|items| {
        let items = items.borrow_mut(); // Removed mut from items
        let total_items = items.len();
        
        // Check if an MCP with the same name already exists
        for i in 0..total_items {
            let existing = items.get(i).unwrap();
            if existing.name == mcp.name {
                return Err(format!("MCP with name '{}' already exists", mcp.name));
            }
        }
        
        // If name is unique, add the new MCP
        let index = items.len();
        let mut mcp_item = mcp.clone();
        mcp_item.id = index;
        items.push(&mcp_item).unwrap();
        
        // Create owner index entry
        USER_MCP_INDEX.with(|user_index| {
            let mut user_index = user_index.borrow_mut();
            let key = UserMcpKey { 
                owner: mcp_item.owner.clone(), 
                item_id: index 
            };
            user_index.insert(key, ());
        });
        
        Ok(index)
    })
}

/// Get an MCP item by index
pub fn get_mcp_item(index: u64) -> Option<McpItem> {
    MCP_ITEMS.with(|items| {
        let items = items.borrow();
        if index < items.len() {
            Some(items.get(index).unwrap())
        } else {
            None
        }
    })
}

/// Get all MCP items
pub fn get_all_mcp_items() -> Vec<McpItem> {
    MCP_ITEMS.with(|items| {
        let items = items.borrow();
        let mut result = Vec::new();
        for i in 0..items.len() {
            result.push(items.get(i).unwrap());
        }
        result
    })
}

/// Get all MCP items owned by a specific user
pub fn get_user_mcp_items(owner: String) -> Vec<McpItem> {
    let mut result = Vec::new();
    
    USER_MCP_INDEX.with(|index| {
        let index = index.borrow();
        
        // Create range bounds for this user
        let start_key = UserMcpKey { owner: owner.clone(), item_id: 0 };
        let end_key = UserMcpKey { owner: owner.clone(), item_id: u64::MAX };
        
        // Get all items in range
        for (key, _) in index.range(start_key..=end_key) {
            if let Some(item) = get_mcp_item(key.item_id) {
                result.push(item);
            }
        }
    });
    
    result
}

/// Update an existing MCP item
pub fn update_mcp_item(index: u64, mut mcp: McpItem) -> Result<(), String> {
    MCP_ITEMS.with(|items| {
        let items = items.borrow_mut(); // Removed mut from items
        if index < items.len() {
            let existing = items.get(index).unwrap();
            
            // Check if the caller is the owner
            if existing.owner != mcp.owner {
                return Err("Only the owner can update this item".to_string());
            }
            
            // Keep the ID and owner
            mcp.id = index;
            
            items.set(index, &mcp);
            Ok(())
        } else {
            Err("Index out of bounds".to_string())
        }
    })
}

/// Get MCP items with pagination
pub fn get_mcp_items_paginated(offset: u64, limit: usize) -> Vec<McpItem> {
    MCP_ITEMS.with(|items| {
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

/// Get MCP items for a specific user with pagination
pub fn get_user_mcp_items_paginated(owner: String, offset: u64, limit: usize) -> Vec<McpItem> {
    let user_items = get_user_mcp_items(owner);
    
    if offset >= user_items.len() as u64 {
        return Vec::new();
    }
    
    let end = std::cmp::min(offset as usize + limit, user_items.len());
    user_items[offset as usize..end].to_vec()
}

/// Get an MCP item by name
pub fn get_mcp_item_by_name(name: String) -> Option<McpItem> {
    MCP_ITEMS.with(|items| {
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
