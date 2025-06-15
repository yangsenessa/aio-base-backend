use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::stable_mem_storage::{MCP_ITEMS, USER_MCP_INDEX, MCP_STACK_RECORDS};

type Memory = VirtualMemory<DefaultMemoryImpl>;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct McpItem {
    pub id: u64,  // nat64 in Candid, must be non-optional
    pub name: String,  // text in Candid
    pub description: String,  // text in Candid
    pub author: String,  // text in Candid
    pub owner: String,  // text in Candid
    pub git_repo: String,  // text in Candid
    pub exec_file: Option<String>,  // opt text in Candid
    pub homepage: Option<String>,  // opt text in Candid
    pub remote_endpoint: Option<String>,  // opt text in Candid
    pub mcp_type: String,  // text in Candid
    pub community_body: Option<String>,  // opt text in Candid
    pub resources: bool,  // bool in Candid
    pub prompts: bool,  // bool in Candid
    pub tools: bool,  // bool in Candid
    pub sampling: bool,  // bool in Candid
}

impl Default for McpItem {
    fn default() -> Self {
        Self {
            id: 1,  // Start with 1 instead of 0
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
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
pub enum StackStatus {
    Stacked,
    Unstacked,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct McpStackRecord {
    pub principal_id: String,
    pub mcp_name: String,
    pub stack_time: u64,
    pub stack_amount: u64,
    pub stack_status: StackStatus,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StackPositionRecord {
    pub id: u64,
    pub mcp_name: String,
    pub stack_amount: u64,
}

// Define the key for user data association
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserMcpKey {
    pub owner: String,
    pub mcp_name: String,  // Changed from item_id to mcp_name
}

impl ic_stable_structures::Storable for UserMcpKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner, &self.mcp_name).expect("Failed to encode UserMcpKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, mcp_name) = Decode!(bytes.as_ref(), String, String).expect("Failed to decode UserMcpKey");
        Self { owner, mcp_name }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for McpItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        ic_cdk::println!("[DEBUG] Storable::to_bytes - Input item: {:?}", self);
        
        // Ensure id is set before encoding
        let mut item = self.clone();
        if item.id == 0 {
            MCP_ITEMS.with(|items| {
                item.id = items.borrow().len() as u64 + 1;
            });
        }
        
        // Use the struct directly for encoding
        let bytes = Encode!(&item).expect("Failed to encode McpItem");
        ic_cdk::println!("[DEBUG] Storable::to_bytes - Encoded bytes length: {}", bytes.len());
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        ic_cdk::println!("[DEBUG] Storable::from_bytes - Input bytes length: {}", bytes.len());
        
        // Try to decode as the struct directly
        match Decode!(bytes.as_ref(), Self) {
            Ok(item) => {
                ic_cdk::println!("[DEBUG] Storable::from_bytes - Successfully decoded: {:?}", item);
                item
            },
            Err(e) => {
                ic_cdk::println!("[ERROR] Storable::from_bytes - Failed to decode: {:?}", e);
                ic_cdk::println!("[ERROR] Storable::from_bytes - Raw bytes: {:?}", bytes);
                
                // If decoding fails, try to create a default item with the stored name
                if bytes.len() > 0 {
                    // Try to extract the name from the bytes if possible
                    if let Ok(name) = String::from_utf8(bytes[..].to_vec()) {
                        if !name.is_empty() {
                            let mut item = McpItem::default();
                            item.name = name;
                            return item;
                        }
                    }
                }
                
                McpItem::default()
            }
        }
    }
        
    const BOUND: Bound = Bound::Bounded { max_size: 20000 * 1024, is_fixed_size: false };
}

impl ic_stable_structures::Storable for McpStackRecord {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).expect("Failed to encode McpStackRecord"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode McpStackRecord")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 4, is_fixed_size: false };
}

impl ic_stable_structures::Storable for StackStatus {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self).expect("Failed to encode StackStatus"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).expect("Failed to decode StackStatus")
    }

    const BOUND: Bound = Bound::Bounded { max_size: 256, is_fixed_size: false };
}

/// Add a new MCP item to the storage
pub fn add_mcp_item(mcp: McpItem, caller_id: String) -> Result<String, String> {
    ic_cdk::println!("[DEBUG] add_mcp_item called with name='{}', caller_id='{}'", mcp.name, caller_id);
    
    // Validate required fields
    if mcp.name.trim().is_empty() {
        return Err("MCP name cannot be empty".to_string());
    }
    
    if mcp.description.trim().is_empty() {
        return Err("MCP description cannot be empty".to_string());
    }
    
    if mcp.author.trim().is_empty() {
        return Err("MCP author cannot be empty".to_string());
    }
    
    if mcp.git_repo.trim().is_empty() {
        return Err("MCP git repository cannot be empty".to_string());
    }
    
    // Validate MCP type
    if !["stdio", "http", "sse"].contains(&mcp.mcp_type.as_str()) {
        return Err("Invalid MCP type. Must be one of: stdio, http, sse".to_string());
    }
    
    // Validate git repository URL format
    if !mcp.git_repo.starts_with("http://") && !mcp.git_repo.starts_with("https://") {
        return Err("Git repository must be a valid HTTP(S) URL".to_string());
    }
    
    // Validate homepage URL if provided
    if let Some(homepage) = &mcp.homepage {
        if !homepage.trim().is_empty() && !homepage.starts_with("http://") && !homepage.starts_with("https://") {
            return Err("Homepage must be a valid HTTP(S) URL".to_string());
        }
    }
    
    // Validate remote endpoint URL if provided
    if let Some(endpoint) = &mcp.remote_endpoint {
        if !endpoint.trim().is_empty() && !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            return Err("Remote endpoint must be a valid HTTP(S) URL".to_string());
        }
    }
    
    // Validate exec_file if provided
    if let Some(exec_file) = &mcp.exec_file {
        if exec_file.trim().is_empty() {
            return Err("Exec file path cannot be empty if provided".to_string());
        }
    }
    
    // Validate community_body if provided
    if let Some(body) = &mcp.community_body {
        if body.trim().is_empty() {
            return Err("Community body cannot be empty if provided".to_string());
        }
    }

    MCP_ITEMS.with(|items| {
        let mut items = items.borrow_mut();
        
        // Check if MCP with same name already exists
        if items.contains_key(&mcp.name) {
            return Err(format!("MCP with name '{}' already exists", mcp.name));
        }
        
        let mut mcp_item = mcp.clone();
        mcp_item.owner = caller_id.clone();
        
        // Set id to current length + 1 to ensure it's never 0
        mcp_item.id = items.len() as u64 + 1;
        
        ic_cdk::println!("[DEBUG] Adding MCP item with id={}, name='{}', owner='{}'", mcp_item.id, mcp_item.name, mcp_item.owner);
        
        // Insert the new item
        items.insert(mcp_item.name.clone(), mcp_item.clone());
        
        // Create owner index entry
        USER_MCP_INDEX.with(|user_index| {
            let mut user_index = user_index.borrow_mut();
            let key = UserMcpKey { 
                owner: mcp_item.owner.clone(), 
                mcp_name: mcp_item.name.clone()
            };
            user_index.insert(key, ());
        });
        
        Ok(mcp_item.name)  // Return the name as the identifier
    })
}

/// Get an MCP item by name
pub fn get_mcp_item(name: String) -> Option<McpItem> {
    MCP_ITEMS.with(|items| {
        items.borrow().get(&name)
    })
}

/// Get all MCP items
pub fn get_all_mcp_items() -> Vec<McpItem> {
    MCP_ITEMS.with(|items| {
        items.borrow().iter().map(|(_, item)| item).collect()
    })
}

/// Get all MCP items owned by a specific user
pub fn get_user_mcp_items(owner: String) -> Vec<McpItem> {
    let mut result = Vec::new();
    
    USER_MCP_INDEX.with(|index| {
        let index = index.borrow();
        
        // Create range bounds for this user
        let start_key = UserMcpKey { owner: owner.clone(), mcp_name: String::new() };
        let end_key = UserMcpKey { owner: owner.clone(), mcp_name: String::from_utf8(vec![255; 100]).unwrap() };
        
        // Get all items in range
        for (key, _) in index.range(start_key..=end_key) {
            if let Some(item) = get_mcp_item(key.mcp_name.clone()) {
                result.push(item);
            }
        }
    });
    
    result
}

/// Update an existing MCP item
pub fn update_mcp_item(name: String, mut mcp: McpItem) -> Result<(), String> {
    MCP_ITEMS.with(|items| {
        let mut items = items.borrow_mut();
        
        // Check if item exists
        if !items.contains_key(&name) {
            return Err(format!("MCP with name '{}' not found", name));
        }
        
        let existing = items.get(&name).unwrap();
        
        // Check if the caller is the owner
        if existing.owner != mcp.owner {
            return Err("Only the owner can update this item".to_string());
        }
        
        // Keep the name, owner, and id from the existing item
        mcp.name = name.clone();
        mcp.id = existing.id;  // Preserve the existing id
        
        items.insert(name, mcp);
        Ok(())
    })
}

/// Get MCP items with pagination
pub fn get_mcp_items_paginated(offset: u64, limit: u64) -> Vec<McpItem> {
    ic_cdk::println!("[DEBUG] get_mcp_items_paginated called with offset={}, limit={}", offset, limit);
    
    MCP_ITEMS.with(|items| {
        let items = items.borrow();
        let total_items = items.len() as u64;
        
        ic_cdk::println!("[DEBUG] total items {}", total_items);
        
        if offset >= total_items {
            return Vec::new();
        }
        
        if limit == 0 {
            ic_cdk::println!("[DEBUG] Limit is 0");
            return Vec::new();
        }
        
        let end = std::cmp::min(offset + limit, total_items);
        ic_cdk::println!("[DEBUG] offset {} end {}", offset, end);
        
        // Get all keys first
        let keys: Vec<String> = items.iter().map(|(key, _)| key.clone()).collect();
        ic_cdk::println!("[DEBUG] Total keys: {}", keys.len());
        
        // Get the slice of keys we need
        let keys_slice = &keys[offset as usize..end as usize];
        ic_cdk::println!("[DEBUG] Keys slice length: {}", keys_slice.len());
        
        // Get items by key and add to result
        let mut result = Vec::new();
        for (index, key) in keys_slice.iter().enumerate() {
            ic_cdk::println!("[DEBUG] Processing item at index {}: {}", index, key);
            if let Some(item) = items.get(key) {
                let mut item = item.clone();
                item.id = (offset + index as u64) + 1;
                result.push(item);
            }
        }
        
        ic_cdk::println!("[DEBUG] Returning {} items", result.len());
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

/// Delete an MCP item by name
pub fn delete_mcp_item(name: String) -> Result<(), String> {
    MCP_ITEMS.with(|items| {
        let mut items = items.borrow_mut();
        
        // Check if item exists
        if !items.contains_key(&name) {
            return Err(format!("MCP with name '{}' not found", name));
        }
        
        // Get the item before removing it
        let item = items.get(&name).unwrap();
        
        // Remove from USER_MCP_INDEX
        USER_MCP_INDEX.with(|user_index| {
            let mut user_index = user_index.borrow_mut();
            let key = UserMcpKey { 
                owner: item.owner.clone(), 
                mcp_name: name.clone()  // Use mcp_name instead of item_id
            };
            user_index.remove(&key);
        });
        
        // Remove the item
        items.remove(&name);
        Ok(())
    })
}

/// Create a stack record for an MCP
pub fn stack_mcp(mcp_name: String, principal_id: String, stack_amount: u64) -> Result<(), String> {
    // Get the current timestamp
    let stack_time = ic_cdk::api::time();
    
    // Create a new stack record
    let stack_record = McpStackRecord {
        principal_id,
        mcp_name,
        stack_time,
        stack_amount,
        stack_status: StackStatus::Stacked,
    };

    // Store the stack record
    MCP_STACK_RECORDS.with(|records| {
        let mut records = records.borrow_mut();
        let record_id = records.len() as u64;
        records.insert(record_id, stack_record);
        Ok(())
    })
}

/// Create an unstack record for an MCP
pub fn unstack_mcp(mcp_name: String, principal_id: String, stack_amount: u64) -> Result<(), String> {
    // Get the current timestamp
    let stack_time = ic_cdk::api::time();
    
    // Create a new unstack record
    let unstack_record = McpStackRecord {
        principal_id,
        mcp_name,
        stack_time,
        stack_amount,
        stack_status: StackStatus::Unstacked,
    };

    // Store the unstack record
    MCP_STACK_RECORDS.with(|records| {
        let mut records = records.borrow_mut();
        let record_id = records.len() as u64;
        records.insert(record_id, unstack_record);
        Ok(())
    })
}

/// Get paginated stack records for a specific MCP, ordered by stack_time desc and stack_status (Stacked first)
pub fn get_mcp_stack_records_paginated(mcp_name: String, offset: u64, limit: u64) -> Vec<McpStackRecord> {
    MCP_STACK_RECORDS.with(|records| {
        let records = records.borrow();
        
        // Collect and filter records for the specific MCP
        let mut filtered_records: Vec<McpStackRecord> = records
            .iter()
            .filter(|(_, record)| record.mcp_name == mcp_name)
            .map(|(_, record)| record.clone())
            .collect();
        
        // Sort by stack_time (desc) and stack_status (Stacked first)
        filtered_records.sort_by(|a, b| {
            match (a.stack_status, b.stack_status) {
                (StackStatus::Stacked, StackStatus::Unstacked) => std::cmp::Ordering::Less,
                (StackStatus::Unstacked, StackStatus::Stacked) => std::cmp::Ordering::Greater,
                _ => b.stack_time.cmp(&a.stack_time), // Descending order for stack_time
            }
        });
        
        // Apply pagination
        let start = offset as usize;
        let end = std::cmp::min(start + limit as usize, filtered_records.len());
        
        if start >= filtered_records.len() {
            return Vec::new();
        }
        
        filtered_records[start..end].to_vec()
    })
}

/// Get total stacked credits across all MCPs
pub fn get_total_stacked_credits() -> u64 {
    MCP_STACK_RECORDS.with(|records| {
        let records = records.borrow();
        if records.is_empty() {
            return 0;
        }
        
        records.iter()
            .filter(|(_, record)| record.stack_status == StackStatus::Stacked)
            .map(|(_, record)| record.stack_amount)
            .sum()
    })
}

/// Get stacked records grouped by MCP name with total stack amount
pub fn get_stacked_record_group_by_stack_amount() -> Vec<StackPositionRecord> {
    MCP_STACK_RECORDS.with(|records| {
        let records = records.borrow();
        if records.is_empty() {
            return Vec::new();
        }
        
        // Create a HashMap to store the sum of stack amounts for each MCP
        let mut mcp_totals: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        
        // Sum up stack amounts for each MCP
        records.iter()
            .filter(|(_, record)| record.stack_status == StackStatus::Stacked)
            .for_each(|(_, record)| {
                *mcp_totals.entry(record.mcp_name.clone())
                    .or_insert(0) += record.stack_amount;
            });
        
        // Convert HashMap to Vec<StackPositionRecord>
        mcp_totals.into_iter()
            .enumerate()
            .map(|(index, (mcp_name, stack_amount))| StackPositionRecord {
                id: (index + 1) as u64,
                mcp_name,
                stack_amount,
            })
            .collect()
    })
}
