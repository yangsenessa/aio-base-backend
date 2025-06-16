mod agent_asset_types;
mod mcp_asset_types;
mod aio_workledger_types;
mod aio_invert_index_types;
mod aio_protocal_types;
mod account_storage;
mod trace_storage;
pub mod mining_reword;
pub mod token_economy_types;
pub mod token_economy;
pub mod stable_mem_storage;

use agent_asset_types::AgentItem;
use mcp_asset_types::{McpItem, McpStackRecord};
use trace_storage::{TraceLog, IOValue};
use ic_cdk::caller;
use aio_protocal_types::AioIndexManager;
use serde_json;
use icrc_ledger_types::{icrc1::account::Account, icrc1::transfer::TransferArg};
use num_traits::ToPrimitive;
use token_economy_types::{
    EmissionPolicy, TokenGrant, TokenInfo,
    TokenActivity, TokenActivityType,
    CreditActivity, CreditActivityType,
    TransferStatus as TokenTransferStatus,
    AccountInfo, TokenGrantStatus, GrantPolicy,
    NewMcpGrant
};
use token_economy::{record_token_activity, record_credit_activity};
use crate::stable_mem_storage::INVERTED_INDEX_STORE;
use candid::{CandidType, Deserialize};
use ic_cdk_timers::TimerId;
use std::time::Duration;
use std::cell::RefCell;
use candid::Principal;

pub use account_storage::*;
pub use trace_storage::*;
pub use mining_reword::*;

// add timer id storage
thread_local! {
    static MINING_TIMER_ID: RefCell<Option<TimerId>> = RefCell::new(None);
}

// add dispatch_mining_rewards function
#[ic_cdk::update]
fn dispatch_mining_rewards() -> Result<(), String> {
    ic_cdk::println!("Starting mining rewards dispatch...");
    
    // check if there is already a timer running
    let timer_exists = MINING_TIMER_ID.with(|timer_id| {
        timer_id.borrow().is_some()
    });
    
    if timer_exists {
        return Err("Mining rewards dispatch is already running".to_string());
    }
    
    // set timer, run once per day
    let timer_id = ic_cdk_timers::set_timer_interval(Duration::from_secs(5 * 60), || {
        ic_cdk::println!("Executing daily mining rewards calculation...");
        match mining_reword::perdic_mining() {
            Ok(_) => ic_cdk::println!("Mining rewards calculation completed"),
            Err(e) => ic_cdk::println!("Mining rewards calculation failed: {}", e),
        }
    });
    
    // store timer id
    MINING_TIMER_ID.with(|id| {
        *id.borrow_mut() = Some(timer_id);
    });
    
    ic_cdk::println!("Mining rewards dispatch has been started");
    Ok(())
}

// add stop mining rewards function
#[ic_cdk::update]
fn stop_mining_rewards() -> Result<(), String> {
    ic_cdk::println!("Stopping mining rewards dispatch...");
    
    MINING_TIMER_ID.with(|timer_id| {
        if let Some(id) = timer_id.borrow_mut().take() {
            ic_cdk_timers::clear_timer(id);
            ic_cdk::println!("Mining rewards dispatch has been stopped");
            Ok(())
        } else {
            Err("No mining rewards dispatch is currently running".to_string())
        }
    })
}

// Store inverted index
#[ic_cdk::update]
fn store_inverted_index(mcp_name: String, json_str: String) -> Result<(), String> {
    ic_cdk::println!("CALL[store_inverted_index] Input: {}", json_str);
    ic_cdk::println!("MCP Name: {}", mcp_name);
    aio_invert_index_types::validate_json_str(&json_str)
        .map_err(|e| format!("Validation failed: {}", e))?;
    // Parse JSON string to Value
    let mut json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // If array, iterate each object and update mcp_name field
    if let serde_json::Value::Array(ref mut array) = json_value {
        for item in array {
            if let serde_json::Value::Object(ref mut map) = item {
                if let Some(value) = map.get_mut("mcp_name") {
                    *value = serde_json::Value::String(mcp_name.clone());
                }
            }
        }
    }
    
    // update json_str
    let json_str = serde_json::to_string(&json_value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    
    
    // store inverted index
    let result = aio_invert_index_types::store_inverted_index(json_str);
    ic_cdk::println!("CALL[store_inverted_index] Output: {:?}", result);
    result
}

// Get all inverted index items
#[ic_cdk::query]
fn get_all_inverted_index_items() -> String {
    ic_cdk::println!("CALL[get_all_inverted_index_items] Input: none");
    let result = aio_invert_index_types::get_all_inverted_index_items();
    ic_cdk::println!("CALL[get_all_inverted_index_items] Output: {} items", result.len());
    result
}

// Get all keywords
#[ic_cdk::query]
fn get_all_keywords() -> String {
    ic_cdk::println!("CALL[get_all_keywords] Input: none");
    let result = aio_invert_index_types::get_all_keywords();
    ic_cdk::println!("CALL[get_all_keywords] Output: {} ", result);
    result
}

// Find index items by keyword
#[ic_cdk::query]
fn find_inverted_index_by_keyword(keyword: String) -> String {
    ic_cdk::println!("CALL[find_inverted_index_by_keyword] Input: keyword={}", keyword);
    let result = aio_invert_index_types::find_inverted_index_by_keyword(keyword);
    ic_cdk::println!("CALL[find_inverted_index_by_keyword] Output: {} items", result.len());
    result
}

// Find index items by keyword group
#[ic_cdk::query]
fn find_inverted_index_by_group(group: String) -> String {
    ic_cdk::println!("CALL[find_inverted_index_by_group] Input: group={}", group);
    let result = aio_invert_index_types::find_inverted_index_by_group(group);
    ic_cdk::println!("CALL[find_inverted_index_by_group] Output: {} items", result.len());
    result
}

// Find index items by MCP name
#[ic_cdk::query]
fn find_inverted_index_by_mcp(mcp_name: String) -> String {
    ic_cdk::println!("CALL[find_inverted_index_by_mcp] Input: mcp_name={}", mcp_name);
    let result = aio_invert_index_types::find_inverted_index_by_mcp(mcp_name);
    ic_cdk::println!("CALL[find_inverted_index_by_mcp] Output: {} items", result.len());
    result
}

// Find index items by confidence threshold
#[ic_cdk::query]
fn find_inverted_index_by_confidence(min_confidence: f32) -> String {
    ic_cdk::println!("CALL[find_inverted_index_by_confidence] Input: min_confidence={}", min_confidence);
    let result = aio_invert_index_types::find_inverted_index_by_confidence(min_confidence);
    ic_cdk::println!("CALL[find_inverted_index_by_confidence] Output: {} items", result.len());
    result
}

// Find index items by multiple keywords with confidence threshold
#[ic_cdk::query]
fn find_inverted_index_by_keywords(keywords: Vec<String>, min_confidence: f32) -> String {
    ic_cdk::println!("CALL[find_inverted_index_by_keywords] Input: keywords={:?}, min_confidence={}", keywords, min_confidence);
    let result = aio_invert_index_types::find_inverted_index_by_keywords(keywords, min_confidence);
    ic_cdk::println!("CALL[find_inverted_index_by_keywords] Output: {} items", result.len());
    result
}

// Delete all index items for a specific MCP
#[ic_cdk::update]
fn delete_inverted_index_by_mcp(mcp_name: String) -> Result<(), String> {
    aio_invert_index_types::delete_inverted_index_by_mcp(mcp_name)
}

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
fn get_mcp_item(name: String) -> Option<McpItem> {
    ic_cdk::println!("CALL[get_mcp_item] Input: name={}", name);
    let result = mcp_asset_types::get_mcp_item(name);
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
fn get_mcp_items_paginated(offset: u64, limit: u64) -> Vec<McpItem> {
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
    let result = mcp_asset_types::get_mcp_item(name);
    ic_cdk::println!("CALL[get_mcp_item_by_name] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::update]
fn add_mcp_item(mcp: McpItem, principalid: String) -> Result<String, String> {
    let caller_id = principalid;
    ic_cdk::println!("CALL[add_mcp_item] Input: caller_id={}, mcp={:?}", caller_id, mcp);
    let result = mcp_asset_types::add_mcp_item(mcp, caller_id);
    ic_cdk::println!("CALL[add_mcp_item] Output: {:?}", result);
    result
}

#[ic_cdk::update]
fn update_mcp_item(name: String, mut mcp: McpItem) -> Result<(), String> {
    let caller_id = caller().to_string();
    ic_cdk::println!("CALL[update_mcp_item] Input: caller_id={}, name={}, mcp={:?}", caller_id, name, mcp);
    mcp.owner = caller_id;
    let result = mcp_asset_types::update_mcp_item(name, mcp);
    ic_cdk::println!("CALL[update_mcp_item] Output: {:?}", result);
    result
}

#[ic_cdk::update]
fn delete_mcp_item(name: String) -> Result<(), String> {
    ic_cdk::println!("CALL[delete_mcp_item] Input: name={}", name);
    
    // First delete the MCP item
    let delete_result = mcp_asset_types::delete_mcp_item(name.clone());
    
    if delete_result.is_ok() {
        // Delete the inverted index
        ic_cdk::println!("CALL[delete_mcp_item] Deleting inverted index for MCP: {}", name);
        let index_result = aio_invert_index_types::delete_inverted_index_by_mcp(name.clone());
        if index_result.is_err() {
            ic_cdk::println!("Warning: Failed to delete inverted index for MCP: {}", name);
            // We don't return error here as the MCP was successfully deleted
        }

        // Delete the index info from aio_protocal_types
        let manager = AioIndexManager::new();
        let protocol_result = manager.delete(&name);
        ic_cdk::println!("CALL[delete_mcp_item] Deleting index info from aio_protocal_types for MCP: {}", name);
        if protocol_result.is_err() {
            ic_cdk::println!("Warning: Failed to delete index info from aio_protocal_types for MCP: {}", name);
            // We don't return error here as the MCP was successfully deleted
        }
    }
    
    ic_cdk::println!("CALL[delete_mcp_item] Output: {:?}", delete_result);
    delete_result
}

// ==== Work Ledger API - Trace System ====

#[ic_cdk::query]
fn get_trace(trace_id: String) -> Option<TraceLog> {
    ic_cdk::println!("CALL[get_trace] Input: trace_id={}", trace_id);
    let result = trace_storage::get_trace_by_id(trace_id);
    ic_cdk::println!("CALL[get_trace] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::query]
fn get_trace_by_context(context_id: String) -> Option<TraceLog> {
    ic_cdk::println!("CALL[get_trace_by_context] Input: context_id={}", context_id);
    let result = trace_storage::get_trace_by_context_id(context_id);
    ic_cdk::println!("CALL[get_trace_by_context] Output: exists={}", result.is_some());
    result
}

#[ic_cdk::query]
fn get_all_traces() -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_all_traces] Input: none");
    let result = trace_storage::get_all_trace_logs();
    ic_cdk::println!("CALL[get_all_traces] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_paginated] Input: offset={}, limit={}", offset, limit);
    let result = trace_storage::get_traces_paginated(offset, limit as u64);
    ic_cdk::println!("CALL[get_traces_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_by_protocol(protocol: String) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_by_protocol] Input: protocol={}", protocol);
    let result = trace_storage::get_traces_by_protocol_name(protocol);
    ic_cdk::println!("CALL[get_traces_by_protocol] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_by_method(method: String) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_by_method] Input: method={}", method);
    let result = trace_storage::get_traces_by_method_name(method);
    ic_cdk::println!("CALL[get_traces_by_method] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_by_status(status: String) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_by_status] Input: status={}", status);
    let result = trace_storage::get_traces_by_status(status, 0, u64::MAX);
    ic_cdk::println!("CALL[get_traces_by_status] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_by_status_paginated(status: String, offset: u64, limit: u64) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_by_status_paginated] Input: status={}, offset={}, limit={}", status, offset, limit);
    let result = trace_storage::get_traces_by_status(status, offset, limit);
    ic_cdk::println!("CALL[get_traces_by_status_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_with_filters(
    protocols: Option<Vec<String>>,
    methods: Option<Vec<String>>,
    statuses: Option<Vec<String>>,
) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_with_filters] Input: protocols={:?}, methods={:?}, statuses={:?}", protocols, methods, statuses);
    let result = trace_storage::get_traces_with_filters(
        protocols.unwrap_or_default(),
        methods.unwrap_or_default(),
        statuses.unwrap_or_default(),
        Vec::new(), // owners
        Vec::new(), // time_ranges
        Vec::new(), // amount_ranges
        Vec::new(), // status_ranges
        u64::MAX,   // limit
    );
    ic_cdk::println!("CALL[get_traces_with_filters] Output: count={}", result.len());
    result
}

#[derive(CandidType, Deserialize)]
struct TraceStatisticsResult {
    total_count: u64,
    success_count: u64,
    error_count: u64,
}

#[ic_cdk::query]
fn get_traces_statistics() -> TraceStatistics {
    ic_cdk::println!("CALL[get_traces_statistics] Input: none");
    let result = trace_storage::get_traces_statistics(0, u64::MAX, u64::MAX);
    ic_cdk::println!("CALL[get_traces_statistics] Output: total_count={}, success_count={}, error_count={}", 
        result.total_count, result.success_count, result.error_count);
    result
}

#[ic_cdk::update]
fn record_trace_call(
    trace_id: String,
    context_id: String,
    protocol: String,
    agent: String,
    call_type: String,
    method: String,
    input: IOValue,
    output: IOValue,
    status: String,
    error_message: Option<String>,
) -> Result<(), String> {
    ic_cdk::println!("CALL[record_trace_call] Input: trace_id={}, context_id={}, protocol={}, method={}", trace_id, context_id, protocol, method);
    let result = trace_storage::record_trace_call(
        trace_id,
        context_id,
        protocol,
        agent,
        call_type,
        method,
        input,
        output,
        status,
        error_message,
    );
    ic_cdk::println!("CALL[record_trace_call] Output: {:?}", result);
    result
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
                    ic_cdk::println!("CALL[export_aio_index_to_json] Output: Success: {}", json);
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

// Find the most suitable index item by keywords with strategy
#[ic_cdk::query]
fn revert_Index_find_by_keywords_strategy(keywords: Vec<String>) -> String {
    ic_cdk::println!("CALL[revert_Index_find_by_keywords_strategy] Input: keywords={:?}", keywords);
    let result = INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_keywords_strategy(&keywords)
    });
    
    // Convert result to JSON string
    let json_result = match result {
        Some(item) => {
            let json = serde_json::to_string(&item).unwrap_or_else(|e| {
                ic_cdk::println!("Error serializing result: {}", e);
                "{}".to_string()
            });
            ic_cdk::println!("Found matching item: {}", json);
            json
        },
        None => {
            ic_cdk::println!("No matching items found");
            "{}".to_string()
        }
    };
    
    ic_cdk::println!("CALL[revert_Index_find_by_keywords_strategy] Output: {}", json_result);
    json_result
}

// ==== Finance API ====

#[ic_cdk::update]
async fn get_account_info(principal_id: String) -> Option<AccountInfo> {
    token_economy::get_account_info(principal_id).await
}

#[ic_cdk::update]
fn add_account(principal_id: String) -> Result<AccountInfo, String> {
    ic_cdk::println!("CALL[add_account] Input: principal_id={}", principal_id);
    let result = token_economy::create_account(principal_id);
    ic_cdk::println!("CALL[add_account] Output: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_all_accounts() -> Vec<AccountInfo> {
    account_storage::get_all_accounts()
}

#[ic_cdk::query]
fn get_accounts_paginated(offset: u64, limit: usize) -> Vec<AccountInfo> {
    account_storage::get_accounts_paginated(offset, limit)
}

#[ic_cdk::update]
fn delete_account(principal_id: String) -> Result<(), String> {
    account_storage::delete_account(principal_id)
}

#[ic_cdk::query]
fn get_balance_summary(principal_id: String) -> (u64, u64, u64, u64) {
    token_economy::get_balance_summary(principal_id)
}

#[ic_cdk::update]
fn stack_credit(principal_id: String,mcp_name:String, amount: u64) -> Result<AccountInfo, String> {
    println!("Input: stack_credit - principal_id: {}, amount: {}", principal_id, amount);
    let result = token_economy::stack_credits(principal_id, mcp_name, amount);
    println!("Output: stack_credit - result: {:?}", result);
    result
}

#[ic_cdk::update]
fn unstack_credit(principal_id: String, amount: u64) -> Result<AccountInfo, String> {
    println!("Input: unstack_credit - principal_id: {}, amount: {}", principal_id, amount);
    let result = token_economy::unstack_credits(principal_id, amount);
    println!("Output: unstack_credit - result: {:?}", result);
    result
}

#[ic_cdk::update]
fn add_token_balance(principal_id: String, amount: u64) -> Result<AccountInfo, String> {
    println!("Input: add_token_balance - principal_id: {}, amount: {}", principal_id, amount);
    let result = token_economy::update_account_balance(principal_id, amount as i64, 0);
    println!("Output: add_token_balance - result: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_traces_by_operation(principal_id: String, operation: String) -> Vec<TraceItem> {
    trace_storage::get_traces_by_operation(principal_id, operation)
}

#[ic_cdk::query]
fn get_traces_by_time_period(principal_id: String, time_period: String) -> Vec<TraceItem> {
    trace_storage::get_traces_by_time_period(principal_id, time_period)
}

#[ic_cdk::query]
fn get_traces_sorted(principal_id: String, sort_by: String, ascending: bool) -> Vec<TraceItem> {
    trace_storage::get_traces_sorted(principal_id, sort_by, ascending)
}

// Token Economy API
#[ic_cdk::update]
fn init_emission_policy() {
    token_economy::init_emission_policy();
}

#[ic_cdk::query]
fn calculate_emission(principal_id: String) -> Result<u64, String> {
    token_economy::calculate_emission(&principal_id)
}

#[ic_cdk::query]
fn get_emission_policy() -> Result<EmissionPolicy, String> {
    token_economy::get_emission_policy()
}

#[ic_cdk::update]
fn update_emission_policy(policy: EmissionPolicy) -> Result<(), String> {
    token_economy::update_emission_policy(policy)
}


#[ic_cdk::query]
fn get_token_grant(recipient: String) -> bool {
    token_economy::get_token_grant(&recipient).is_some()
}

#[ic_cdk::query]
fn check_is_newuser(principal_id: String) -> bool {
    token_economy::get_token_grant(&principal_id).is_none()
}


#[ic_cdk::query]
fn get_all_token_grants() -> Vec<TokenGrant> {
    token_economy::get_all_token_grants()
}

#[ic_cdk::query]
fn get_token_grants_paginated(offset: u64, limit: usize) -> Vec<TokenGrant> {
    token_economy::get_token_grants_paginated(offset, limit)
}

#[ic_cdk::query]
fn get_token_grants_by_recipient(recipient: String) -> Vec<TokenGrant> {
    token_economy::get_token_grants_by_recipient(&recipient)
}

#[ic_cdk::query]
fn get_token_grants_by_status(status: String) -> Vec<TokenGrant> {
    let grant_status = match status.as_str() {
        "Pending" => TokenGrantStatus::Pending,
        "Active" => TokenGrantStatus::Active,
        "Completed" => TokenGrantStatus::Completed,
        "Cancelled" => TokenGrantStatus::Cancelled,
        _ => TokenGrantStatus::Pending, // Default to Pending for invalid status
    };
    token_economy::get_token_grants_by_status(&grant_status)
}

#[ic_cdk::query]
fn get_token_grants_count() -> u64 {
    token_economy::get_token_grants_count()
}

#[ic_cdk::query]
fn get_account_token_info(principal_id: String) -> Result<TokenInfo, String> {
    token_economy::get_account_token_info(&principal_id)
}

#[ic_cdk::update]
fn log_credit_usage(principal_id: String, amount: u64, service: String, metadata: Option<String>) -> Result<(), String> {
    token_economy::log_credit_usage(principal_id, amount, service, metadata)
}

// Token Activity API
#[ic_cdk::query]
fn get_token_activities(principal_id: String) -> Vec<TokenActivity> {
    token_economy::get_token_activities(&principal_id)
}

#[ic_cdk::query]
fn get_token_activities_paginated(principal_id: String, offset: u64, limit: usize) -> Vec<TokenActivity> {
    token_economy::get_token_activities_paginated(&principal_id, offset, limit)
}

#[ic_cdk::query]
fn get_token_activities_by_type(principal_id: String, activity_type: TokenActivityType) -> Vec<TokenActivity> {
    token_economy::get_token_activities_by_type(&principal_id, activity_type)
}

#[ic_cdk::query]
fn get_token_activities_by_time_period(principal_id: String, start_time: u64, end_time: u64) -> Vec<TokenActivity> {
    token_economy::get_token_activities_by_time_period(&principal_id, start_time, end_time)
}

#[ic_cdk::query]
fn get_token_activity_statistics(principal_id: String) -> (u64, u64, u64) {
    token_economy::get_token_activity_statistics(&principal_id)
}

// Credit Activity API
#[ic_cdk::query]
fn get_credit_activities(principal_id: String) -> Vec<CreditActivity> {
    token_economy::get_credit_activities(&principal_id)
}

#[ic_cdk::query]
fn get_credit_activities_paginated(principal_id: String, offset: u64, limit: usize) -> Vec<CreditActivity> {
    token_economy::get_credit_activities_paginated(&principal_id, offset, limit)
}

#[ic_cdk::query]
fn get_credit_activities_by_type(principal_id: String, activity_type: CreditActivityType) -> Vec<CreditActivity> {
    token_economy::get_credit_activities_by_type(&principal_id, activity_type)
}

#[ic_cdk::query]
fn get_credit_activities_by_time_period(principal_id: String, start_time: u64, end_time: u64) -> Vec<CreditActivity> {
    token_economy::get_credit_activities_by_time_period(&principal_id, start_time, end_time)
}

#[ic_cdk::query]
fn get_credit_activity_statistics(principal_id: String) -> (u64, u64, u64) {
    token_economy::get_credit_activity_statistics(&principal_id)
}

#[ic_cdk::update]
fn use_credit(principal_id: String, amount: u64, service: String, metadata: Option<String>) -> Result<AccountInfo, String> {
    println!("Input: use_credit - principal_id: {}, amount: {}, service: {}", principal_id, amount, service);
    let result = token_economy::use_credits(principal_id, amount, service, metadata);
    println!("Output: use_credit - result: {:?}", result);
    result
}

#[ic_cdk::update]
fn grant_token(grant: TokenGrant) -> Result<(), String> {
    println!("Input: grant_token - grant: {:?}", grant);
    
    let result = token_economy::create_token_grant(grant.clone())?;
    
    // Record token activity for granting
    let activity = TokenActivity {
        timestamp: ic_cdk::api::time() / 1_000_000,
        from: "system".to_string(),
        to: grant.recipient,
        amount: grant.amount,
        activity_type: TokenActivityType::Grant,
        status: TokenTransferStatus::Completed,
        metadata: Some("Token grant".to_string()),
    };
    record_token_activity(activity)?;
    
    println!("Output: grant_token - result: {:?}", result);
    Ok(result)
}

#[ic_cdk::update]
fn transfer_token(from: String, to: String, amount: u64) -> Result<AccountInfo, String> {
    println!("Input: transfer_token - from: {}, to: {}, amount: {}", from, to, amount);
    let result = token_economy::transfer_tokens(from, to, amount);
    println!("Output: transfer_token - result: {:?}", result);
    result
}

#[ic_cdk::update]
fn init_grant_policy(grant_policy: Option<GrantPolicy>) {
    token_economy::init_grant_policy(grant_policy);
}

#[ic_cdk::update]
fn create_and_claim_newuser_grant(principal_id: String) -> Result<u64, String> {
    println!("Input: create_and_claim_newuser_grant - principal_id: {}", principal_id);
    
    // Step 1: Check if grant exists and its status
    if let Some(grant) = token_economy::get_token_grant(&principal_id) {
        match grant.status {
            TokenGrantStatus::Active => {
                // Step 3: If grant is active, claim it
                let claim_result = token_economy::claim_grant(&principal_id)?;
                println!("Output: create_and_claim_newuser_grant - claimed amount: {}", claim_result);
                Ok(claim_result)
            },
            _ => Err(format!("Grant exists but is not active. Current status: {:?}", grant.status))
        }
    } else {
        // Step 2: No grant exists, create a new one
        let new_grant = TokenGrant {
            recipient: principal_id.clone(),
            amount: 1000, // Default amount for new users
            start_time: ic_cdk::api::time() / 1_000_000,
            claimed_amount: 0,
            status: TokenGrantStatus::Active,
        };
        
        token_economy::create_token_grant(new_grant)?;
        
        // Step 3: Claim the newly created grant
        let claim_result = token_economy::claim_grant(&principal_id)?;
        println!("Output: create_and_claim_newuser_grant - claimed amount: {}", claim_result);
        Ok(claim_result)
    }
}

#[ic_cdk::update]
fn create_and_claim_newmcp_grant(principal_id: String, mcp_name: String) -> Result<u64, String> {
    ic_cdk::println!("Input: create_and_claim_newmcp_grant - principal_id: {}, mcp_name: {}", principal_id, mcp_name);
    
    // First create a new MCP grant
    let new_grant = NewMcpGrant {
        recipient: principal_id.clone(),
        mcp_name: mcp_name.clone(),
        amount: 10000, // Default amount for new MCP
        start_time: ic_cdk::api::time() / 10_000,
        claimed_amount: 0,
        status: TokenGrantStatus::Active,
    };
    
    token_economy::create_mcp_grant(new_grant)?;
    
    // Then claim the grant
    let claim_result = token_economy::claim_mcp_grant_with_mcpname(&principal_id, &mcp_name)?;
    println!("Output: create_and_claim_newmcp_grant - claimed amount: {}", claim_result);
    Ok(claim_result)
}

#[ic_cdk::update]
fn create_mcp_grant(grant: NewMcpGrant) -> Result<(), String> {
    println!("Input: create_mcp_grant - grant: {:?}", grant);
    let result = token_economy::create_mcp_grant(grant);
    println!("Output: create_mcp_grant - result: {:?}", result);
    result
}

#[ic_cdk::update]
fn claim_mcp_grant(principal_id: String) -> Result<u64, String> {
    println!("Input: claim_mcp_grant - principal_id: {}", principal_id);
    let result = token_economy::claim_mcp_grant(&principal_id);
    println!("Output: claim_mcp_grant - result: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_mcp_grant(recipient: String, mcp_name: String) -> Option<NewMcpGrant> {
    println!("Input: get_mcp_grant - recipient: {}, mcp_name: {}", recipient, mcp_name);
    let result = token_economy::get_mcp_grant(&recipient, &mcp_name);
    println!("Output: get_mcp_grant - result: {:?}", result);
    result
}

#[ic_cdk::query]
fn get_all_mcp_grants() -> Vec<NewMcpGrant> {
    println!("Input: get_all_mcp_grants");
    let result = token_economy::get_all_mcp_grants();
    println!("Output: get_all_mcp_grants - count: {}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_grants_paginated(offset: u64, limit: usize) -> Vec<NewMcpGrant> {
    println!("Input: get_mcp_grants_paginated - offset: {}, limit: {}", offset, limit);
    let result = token_economy::get_mcp_grants_paginated(offset, limit);
    println!("Output: get_mcp_grants_paginated - count: {}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_grants_by_recipient(recipient: String) -> Vec<NewMcpGrant> {
    println!("Input: get_mcp_grants_by_recipient - recipient: {}", recipient);
    let result = token_economy::get_mcp_grants_by_recipient(&recipient);
    println!("Output: get_mcp_grants_by_recipient - count: {}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_grants_by_mcp(mcp_name: String) -> Vec<NewMcpGrant> {
    println!("Input: get_mcp_grants_by_mcp - mcp_name: {}", mcp_name);
    let result = token_economy::get_mcp_grants_by_mcp(&mcp_name);
    println!("Output: get_mcp_grants_by_mcp - count: {}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_grants_by_status(status: TokenGrantStatus) -> Vec<NewMcpGrant> {
    println!("Input: get_mcp_grants_by_status - status: {:?}", status);
    let result = token_economy::get_mcp_grants_by_status(&status);
    println!("Output: get_mcp_grants_by_status - count: {}", result.len());
    result
}

#[ic_cdk::query]
fn get_mcp_grants_count() -> u64 {
    println!("Input: get_mcp_grants_count");
    let result = token_economy::get_mcp_grants_count();
    println!("Output: get_mcp_grants_count - count: {}", result);
    result
}

#[ic_cdk::query]
fn get_mcp_stack_records_paginated(mcp_name: String, offset: u64, limit: u64) -> Vec<McpStackRecord> {
    ic_cdk::println!("CALL[get_mcp_stack_records_paginated] Input: mcp_name={}, offset={}, limit={}", mcp_name, offset, limit);
    let result = mcp_asset_types::get_mcp_stack_records_paginated(mcp_name, offset, limit);
    ic_cdk::println!("CALL[get_mcp_stack_records_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_traces_by_agentname_paginated(agent_name: String, offset: u64, limit: u64) -> Vec<TraceLog> {
    ic_cdk::println!("CALL[get_traces_by_agentname_paginated] Input: agent_name={}, offset={}, limit={}", agent_name, offset, limit);
    let result = trace_storage::get_traces_by_agentname_paginated(agent_name, offset, limit);
    ic_cdk::println!("CALL[get_traces_by_agentname_paginated] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn cal_unclaim_rewards(principal_id: String) -> u64 {
    ic_cdk::println!("CALL[cal_unclaim_rewards] Input: principal_id={}", principal_id);
    let principal = Principal::from_text(&principal_id)
        .unwrap_or_else(|_| Principal::anonymous());
    let result = mining_reword::cal_unclaim_rewards(principal);
    ic_cdk::println!("CALL[cal_unclaim_rewards] Output: {}", result);
    result
}

#[ic_cdk::update]
async fn claim_rewards(principal_id: String) -> Result<u64, String> {
    let principal = Principal::from_text(&principal_id)
        .map_err(|e| format!("Invalid principal ID: {}", e))?;
    
    #[derive(CandidType, Deserialize)]
    struct ClaimRewardsResult {
        Ok: Option<u64>,
        Err: Option<String>,
    }
    
    match mining_reword::claim_rewards(principal).await {
        Ok(amount) => Ok(amount),
        Err(e) => Err(e),
    }
}

#[ic_cdk::query]
fn get_total_aiotoken_claimable() -> u64 {
    mining_reword::get_total_aiotoken_claimable()
}

#[ic_cdk::query]
fn get_total_stacked_credits() -> u64 {
    ic_cdk::println!("CALL[get_total_stacked_credits] Input: none");
    let result = mcp_asset_types::get_total_stacked_credits();
    ic_cdk::println!("CALL[get_total_stacked_credits] Output: {}", result);
    result
}

#[ic_cdk::query]
fn get_stacked_record_group_by_stack_amount() -> Vec<mcp_asset_types::StackPositionRecord> {
    ic_cdk::println!("CALL[get_stacked_record_group_by_stack_amount] Input: none");
    let result = mcp_asset_types::get_stacked_record_group_by_stack_amount();
    ic_cdk::println!("CALL[get_stacked_record_group_by_stack_amount] Output: count={}", result.len());
    result
}

#[ic_cdk::query]
fn get_all_mcp_names() -> Vec<String> {
    ic_cdk::println!("CALL[get_all_mcp_names]");
    let result = mcp_asset_types::get_all_mcp_names();
    ic_cdk::println!("CALL[get_all_mcp_names] Output: {:?}", result);
    result
}

