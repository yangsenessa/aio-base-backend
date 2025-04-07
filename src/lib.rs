mod agent_asset_types;
mod mcp_asset_types;
mod aio_workledger_types;
// Import the AioIndexManager
mod aio_protocal_types;

use agent_asset_types::AgentItem;
use candid::types::principal;
use mcp_asset_types::McpItem;
use aio_workledger_types::TraceItem;
use ic_cdk::caller;
use aio_protocal_types::AioIndexManager;
use serde_json;

#[ic_cdk::query]
fn greet(name: String) -> String {
    ic_cdk::println!("CALL[greet] Input: {}", name);
    let result = format!("Hello, {}!", name);
    ic_cdk::println!("CALL[greet] Output: {}", result);
    result
}

// ==== Agent Asset API ====

#[ic_cdk::query]
fn get_agent_item(index: u64) -> Option<AgentItem> {
    ic_cdk::println!("CALL[get_agent_item] Input: index={}", index);
    let result = agent_asset_types::get_agent_item(index);
    ic_cdk::println!("CALL[get_agent_item] Output: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_all_agent_items() -> Vec<AgentItem> {
    ic_cdk::println!("CALL[get_all_agent_items] Input: none");
    let result = agent_asset_types::get_all_agent_items();
    ic_cdk::println!("CALL[get_all_agent_items] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_user_agent_items() -> Vec<AgentItem> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[get_user_agent_items] Input: caller_id={}", caller_id);
    let result = agent_asset_types::get_user_agent_items(caller_id);
    ic_cdk::println!("CALL[get_user_agent_items] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_agent_items_paginated(offset: u64, limit: usize) -> Vec<AgentItem> {
    ic_cdk::println!("CALL[get_agent_items_paginated] Input: offset={}, limit={}", offset, limit);
    let result = agent_asset_types::get_agent_items_paginated(offset, limit);
    ic_cdk::println!("CALL[get_agent_items_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_agent_item_by_name(name: String) -> Option<AgentItem> {
    ic_cdk::println!("CALL[get_agent_item_by_name] Input: name={}", name);
    let result = agent_asset_types::get_agent_item_by_name(name);
    
    // Print the full details of the result
    match &result {
        Some(item) => ic_cdk::println!("CALL[get_agent_item_by_name] Output: Some({:?})", item),
        None => ic_cdk::println!("CALL[get_agent_item_by_name] Output: None"),
    }
    
    result
}

#[ic_cdk::update]
fn add_agent_item(agent: AgentItem, principalid: String) -> Result<u64, String> {
    ic_cdk::println!("CALL[add_agent_item] Input: caller_id={}, agent={:?}", principalid, agent);
    let mut agent_item = agent.clone();
    agent_item.owner = principalid.clone();
    let result = agent_asset_types::add_agent_item(agent_item); // Pass the modified agent with owner
    ic_cdk::println!("CALL[add_agent_item] Output: {:?}", result);
    result
}

#[ic_cdk::update]
fn update_agent_item(index: u64, mut agent: AgentItem) -> Result<(), String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[update_agent_item] Input: caller_id={}, index={}, agent={:?}", caller_id, index, agent);
    agent.owner = caller_id;
    let result = agent_asset_types::update_agent_item(index, agent);
    ic_cdk::println!("CALL[update_agent_item] Output: {:?}", result);
    result
}

// ==== MCP Asset API ====

#[ic_cdk::query]
fn get_mcp_item(index: u64) -> Option<McpItem> {
    ic_cdk::println!("CALL[get_mcp_item] Input: index={}", index);
    let result = mcp_asset_types::get_mcp_item(index);
    ic_cdk::println!("CALL[get_mcp_item] Output: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_all_mcp_items() -> Vec<McpItem> {
    ic_cdk::println!("CALL[get_all_mcp_items] Input: none");
    let result = mcp_asset_types::get_all_mcp_items();
    ic_cdk::println!("CALL[get_all_mcp_items] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_user_mcp_items() -> Vec<McpItem> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[get_user_mcp_items] Input: caller_id={}", caller_id);
    let result = mcp_asset_types::get_user_mcp_items(caller_id);
    ic_cdk::println!("CALL[get_user_mcp_items] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_items_paginated(offset: u64, limit: usize) -> Vec<McpItem> {
    ic_cdk::println!("CALL[get_mcp_items_paginated] Input: offset={}, limit={}", offset, limit);
    let result = mcp_asset_types::get_mcp_items_paginated(offset, limit);
    ic_cdk::println!("CALL[get_mcp_items_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_user_mcp_items_paginated(offset: u64, limit: usize) -> Vec<McpItem> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[get_user_mcp_items_paginated] Input: caller_id={}, offset={}, limit={}", caller_id, offset, limit);
    let result = mcp_asset_types::get_user_mcp_items_paginated(caller_id, offset, limit);
    ic_cdk::println!("CALL[get_user_mcp_items_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_item_by_name(name: String) -> Option<McpItem> {
    ic_cdk::println!("CALL[get_mcp_item_by_name] Input: name={}", name);
    let result = mcp_asset_types::get_mcp_item_by_name(name);
    ic_cdk::println!("CALL[get_mcp_item_by_name] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::update]
fn add_mcp_item(mcp: McpItem, principalid: String) -> Result<u64, String> {
    let caller_id = principalid;
    ic_cdk::println!("CALL[add_mcp_item] Input: caller_id={}, mcp={:?}", caller_id, mcp);
    let result = mcp_asset_types::add_mcp_item(mcp, caller_id);
    ic_cdk::println!("CALL[add_mcp_item] Output: {:?}", result);
    result
}

#[ic_cdk::update]
fn update_mcp_item(index: u64, mut mcp: McpItem) -> Result<(), String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[update_mcp_item] Input: caller_id={}, index={}, mcp={:?}", caller_id, index, mcp);
    mcp.owner = caller_id;
    let result = mcp_asset_types::update_mcp_item(index, mcp);
    ic_cdk::println!("CALL[update_mcp_item] Output: {:?}", result);
    result
}

// ==== Work Ledger API - Trace System ====

#[ic_cdk::query]
fn get_trace(index: u64) -> Option<TraceItem> {
    ic_cdk::println!("CALL[get_trace] Input: index={}", index);
    let result = aio_workledger_types::get_trace(index);
    ic_cdk::println!("CALL[get_trace] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::query]
fn get_trace_by_id(trace_id: String) -> Option<TraceItem> {
    ic_cdk::println!("CALL[get_trace_by_id] Input: trace_id={}", trace_id);
    let result = aio_workledger_types::get_trace_by_id(trace_id);
    ic_cdk::println!("CALL[get_trace_by_id] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::query]
fn get_user_traces() -> Vec<TraceItem> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[get_user_traces] Input: caller_id={}", caller_id);
    let result = aio_workledger_types::get_user_traces(caller_id);
    ic_cdk::println!("CALL[get_user_traces] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_user_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[get_user_traces_paginated] Input: caller_id={}, offset={}, limit={}", caller_id, offset, limit);
    let result = aio_workledger_types::get_user_traces_paginated(caller_id, offset, limit);
    ic_cdk::println!("CALL[get_user_traces_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    ic_cdk::println!("CALL[get_traces_paginated] Input: offset={}, limit={}", offset, limit);
    let result = aio_workledger_types::get_traces_paginated(offset, limit);
    ic_cdk::println!("CALL[get_traces_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::update]
fn add_trace(mut trace: TraceItem) -> Result<u64, String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[add_trace] Input: caller_id={}, trace={:?}", caller_id, trace);
    trace.owner = caller_id;
    trace.created_at = ic_cdk::api::time() / 1_000_000; // Convert nanoseconds to milliseconds
    trace.updated_at = trace.created_at;
    
    let result = aio_workledger_types::add_trace(trace);
    ic_cdk::println!("CALL[add_trace] Output: {:?}", result);
    result
}

// Helper function unchanged
fn time_now_string() -> String {
    let now_nanos = ic_cdk::api::time();
    let now_millis = now_nanos / 1_000_000;
    
    // We don't have the full chrono crate in IC, so manual conversion:
    let seconds = (now_millis / 1000) as i64;
    let days_since_epoch = seconds / 86400;
    
    // Very simple date algorithm for demo purposes
    // Actual implementation would handle leap years properly
    let year = 1970 + (days_since_epoch / 365);
    let month = ((days_since_epoch % 365) / 30) + 1;
    let day = (days_since_epoch % 365) % 30 + 1;
    
    format!("{:04}{:02}{:02}", year, month, day)
}



// ==== AIO Protocol Index API ====

#[ic_cdk::update]
fn create_aio_index_from_json(name:String,json_str: String) -> Result<(), String> {
    ic_cdk::println!("CALL[create_aio_index_from_json] Input: name={}, json_str={}",  name, json_str);
    let manager = AioIndexManager::new();
    let result = manager.create_from_json(&name,&json_str);
    ic_cdk::println!("CALL[create_aio_index_from_json] Output: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_aio_index(id: String) -> Option<aio_protocal_types::AioIndex> {
    ic_cdk::println!("CALL[get_aio_index] Input: id={}", id);
    let manager = AioIndexManager::new();
    let result = manager.read(&id);
    ic_cdk::println!("CALL[get_aio_index] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::query]
fn get_all_aio_indices() -> Vec<aio_protocal_types::AioIndex> {
    ic_cdk::println!("CALL[get_all_aio_indices] Input: none");
    let manager = AioIndexManager::new();
    let result = manager.list_all();
    ic_cdk::println!("CALL[get_all_aio_indices] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_aio_indices_paginated(offset: usize, limit: usize) -> Vec<aio_protocal_types::AioIndex> {
    ic_cdk::println!("CALL[get_aio_indices_paginated] Input: offset={}, limit={}", offset, limit);
    let manager = AioIndexManager::new();
    let result = manager.get_indices_paginated(offset, limit);
    ic_cdk::println!("CALL[get_aio_indices_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn search_aio_indices_by_keyword(keyword: String) -> Vec<aio_protocal_types::AioIndex> {
    ic_cdk::println!("CALL[search_aio_indices_by_keyword] Input: keyword={}", keyword);
    let manager = AioIndexManager::new();
    let result = manager.search_by_keyword(&keyword);
    ic_cdk::println!("CALL[search_aio_indices_by_keyword] Output: count={}", result.len());
    result
}

#[ic_cdk::update]
fn update_aio_index(id: String, json_str: String) -> Result<(), String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[update_aio_index] Input: caller_id={}, id={}", caller_id, id);
    
    // Parse JSON to AioIndex
    let parsed: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => return Err(format!("Invalid JSON: {}", e))
    };
    
    // Create manager and update
    let manager = AioIndexManager::new();
    
    // First verify the index exists
    if let Some(mut index) = manager.read(&id) {
        // Update from parsed JSON
        if let Some(obj) = parsed.as_object() {
            // Update fields as necessary
            if let Some(description) = obj.get("description").and_then(|v| v.as_str()) {
                index.description = description.to_string();
            }
            
            // Additional fields can be updated here...
            
            // Then call update
            let result = manager.update(&id, index);
            ic_cdk::println!("CALL[update_aio_index] Output: {:?}", result);
            result
        } else {
            Err("Invalid JSON: expected object".to_string())
        }
    } else {
        Err(format!("Index with ID {} not found", id))
    }
}

#[ic_cdk::update]
fn delete_aio_index(id: String) -> Result<(), String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[delete_aio_index] Input: caller_id={}, id={}", caller_id, id);
    let manager = AioIndexManager::new();
    let result = manager.delete(&id);
    ic_cdk::println!("CALL[delete_aio_index] Output: {:?}", result);
    result
}

#[ic_cdk::query]
fn export_aio_index_to_json(id: String) -> Result<String, String> {
    ic_cdk::println!("CALL[export_aio_index_to_json] Input: id={}", id);
    let manager = AioIndexManager::new();
    
    // Get the index first
    match manager.read(&id) {
        Some(index) => {
            // Serialize to JSON
            match serde_json::to_string(&index) {
                Ok(json) => {
                    ic_cdk::println!("CALL[export_aio_index_to_json] Output: Success (JSON string)");
                    Ok(json)
                },
                Err(e) => {
                    let error = format!("Failed to serialize index to JSON: {}", e);
                    ic_cdk::println!("CALL[export_aio_index_to_json] Output: Error - {}", error);
                    Err(error)
                }
            }
        },
        None => {
            let error = format!("Index with ID {} not found", id);
            ic_cdk::println!("CALL[export_aio_index_to_json] Output: Error - {}", error);
            Err(error)
        }
    }
}

#[ic_cdk::query]
fn get_aio_indices_count() -> usize {
    ic_cdk::println!("CALL[get_aio_indices_count] Input: none");
    let manager = AioIndexManager::new();
    let result = manager.count();
    ic_cdk::println!("CALL[get_aio_indices_count] Output: {}", result);
    result
}

