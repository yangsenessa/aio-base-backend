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
        Cow::Owned(Encode!(&self.owner, &self.item_id).expect("Failed to encode UserMcpKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, item_id) = Decode!(bytes.as_ref(), String, u64).expect("Failed to decode UserMcpKey");
        Self { owner, item_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for McpItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).expect("Failed to encode McpItem"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode McpItem")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 20000 * 1024, is_fixed_size: false }; // 100KB should be sufficient
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static MCP_ITEMS: RefCell<StableVec<McpItem, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
        ).expect("Failed to initialize MCP items storage")
    );
    
    static USER_MCP_INDEX: RefCell<StableBTreeMap<UserMcpKey, (), Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(7)))
        )
    );
}

/// Add a new MCP item to the storage
pub fn add_mcp_item(mcp: McpItem, caller_id: String) -> Result<u64, String> {
    MCP_ITEMS.with(|items| {
        let items = items.borrow_mut();
        let total_items = items.len();
        
        // Check if an MCP with the same name already exists
        for i in 0..total_items {
            let existing = items.get(i).unwrap();
            if existing.name == mcp.name {
                return Err(format!("MCP with name '{}' already exists", mcp.name));
            }
        }
        ic_cdk::println!("Adding new MCP item: name={}, owner={}", mcp.name, caller_id);
        // If name is unique, add the new MCP
        let index = items.len();
        let mut mcp_item = mcp.clone();
        mcp_item.id = index;
        mcp_item.owner = caller_id; // 使用传入的 caller_id 作为 owner
        items.push(&mcp_item).unwrap();
        ic_cdk::println!("MCP item added: index={}, name={}", index, mcp_item.name);
        ic_cdk::println!("MCP_INDEX item will be added: index={}, owner_name={}", index, mcp_item.owner);
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
            let item = items.get(index).unwrap();
            // Check if it's an empty object
            if item.name.is_empty() {
                None
            } else {
                Some(item)
            }
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
            let item = items.get(i).unwrap();
            // Only add non-empty objects
            if !item.name.is_empty() {
                result.push(item);
            }
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
                // Only add non-empty objects
                if !item.name.is_empty() {
                    result.push(item);
                }
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
            let item = items.get(i).unwrap();
            // Only add non-empty objects
            if !item.name.is_empty() {
                result.push(item);
            }
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
            // Check if it's a non-empty object and name matches
            if !item.name.is_empty() && item.name == name {
                return Some(item);
            }
        }
        None
    })
}

/// Delete an MCP item by name
pub fn delete_mcp_item(name: String) -> Result<(), String> {
    MCP_ITEMS.with(|items| {
        let mut items = items.borrow_mut();
        let total_items = items.len();
        
        // Find the item by name
        let mut found_index = None;
        for i in 0..total_items {
            let item = items.get(i).unwrap();
            if item.name == name {
                found_index = Some(i);
                break;
            }
        }
        
        match found_index {
            Some(index) => {
                // Remove from USER_MCP_INDEX
                USER_MCP_INDEX.with(|user_index| {
                    let mut user_index = user_index.borrow_mut();
                    let item = items.get(index).unwrap();
                    let key = UserMcpKey { 
                        owner: item.owner.clone(), 
                        item_id: index as u64 
                    };
                    user_index.remove(&key);
                });
                
                // Create an empty MCP item to replace the existing one
                let empty_item = McpItem {
                    id: index as u64,
                    name: String::new(),
                    description: String::new(),
                    author: String::new(),
                    owner: String::new(),
                    git_repo: String::new(),
                    exec_file: None,
                    homepage: None,
                    remote_endpoint: None,
                    mcp_type: String::new(),
                    community_body: None,
                    resources: false,
                    prompts: false,
                    tools: false,
                    sampling: false,
                };
                
                // Replace the item with empty one
                items.set(index, &empty_item);
                Ok(())
            },
            None => Err(format!("MCP with name '{}' not found", name))
        }
    })
}
