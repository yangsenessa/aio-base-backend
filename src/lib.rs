mod agent_asset_types;
mod mcp_asset_types;
mod aio_workledger_types;

use agent_asset_types::AgentItem;
use mcp_asset_types::McpItem;
use aio_workledger_types::TraceItem;
use ic_cdk::caller;

#[ic_cdk::query]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

// ==== Agent Asset API ====

#[ic_cdk::query]
fn get_agent_item(index: u64) -> Option<AgentItem> {
    agent_asset_types::get_agent_item(index)
}

#[ic_cdk::query]
fn get_all_agent_items() -> Vec<AgentItem> {
    agent_asset_types::get_all_agent_items()
}

#[ic_cdk::query]
fn get_user_agent_items() -> Vec<AgentItem> {
    let caller_id = caller().to_string();
    agent_asset_types::get_user_agent_items(caller_id)
}

#[ic_cdk::query]
fn get_agent_items_paginated(offset: u64, limit: usize) -> Vec<AgentItem> {
    agent_asset_types::get_agent_items_paginated(offset, limit)
}

#[ic_cdk::query]
fn get_user_agent_items_paginated(offset: u64, limit: usize) -> Vec<AgentItem> {
    let caller_id = caller().to_string();
    agent_asset_types::get_user_agent_items_paginated(caller_id, offset, limit)
}

#[ic_cdk::query]
fn get_agent_item_by_name(name: String) -> Option<AgentItem> {
    agent_asset_types::get_agent_item_by_name(name)
}

#[ic_cdk::update]
fn add_agent_item(mut agent: AgentItem) -> Result<u64, String> {
    let caller_id = caller().to_string();
    agent.owner = caller_id;
    agent_asset_types::add_agent_item(agent)
}

#[ic_cdk::update]
fn update_agent_item(index: u64, mut agent: AgentItem) -> Result<(), String> {
    let caller_id = caller().to_string();
    agent.owner = caller_id;
    agent_asset_types::update_agent_item(index, agent)
}

// ==== MCP Asset API ====

#[ic_cdk::query]
fn get_mcp_item(index: u64) -> Option<McpItem> {
    mcp_asset_types::get_mcp_item(index)
}

#[ic_cdk::query]
fn get_all_mcp_items() -> Vec<McpItem> {
    mcp_asset_types::get_all_mcp_items()
}

#[ic_cdk::query]
fn get_user_mcp_items() -> Vec<McpItem> {
    let caller_id = caller().to_string();
    mcp_asset_types::get_user_mcp_items(caller_id)
}

#[ic_cdk::query]
fn get_mcp_items_paginated(offset: u64, limit: usize) -> Vec<McpItem> {
    mcp_asset_types::get_mcp_items_paginated(offset, limit)
}

#[ic_cdk::query]
fn get_user_mcp_items_paginated(offset: u64, limit: usize) -> Vec<McpItem> {
    let caller_id = caller().to_string();
    mcp_asset_types::get_user_mcp_items_paginated(caller_id, offset, limit)
}

#[ic_cdk::query]
fn get_mcp_item_by_name(name: String) -> Option<McpItem> {
    mcp_asset_types::get_mcp_item_by_name(name)
}

#[ic_cdk::update]
fn add_mcp_item(mut mcp: McpItem) -> Result<u64, String> {
    let caller_id = caller().to_string();
    mcp.owner = caller_id;
    mcp_asset_types::add_mcp_item(mcp)
}

#[ic_cdk::update]
fn update_mcp_item(index: u64, mut mcp: McpItem) -> Result<(), String> {
    let caller_id = caller().to_string();
    mcp.owner = caller_id;
    mcp_asset_types::update_mcp_item(index, mcp)
}

// ==== Work Ledger API ====
// Remove these functions since WorkItem doesn't exist in aio_workledger_types
// These would need to be implemented if the WorkItem functionality is needed

// ==== Work Ledger API - Trace System ====

#[ic_cdk::query]
fn get_trace(index: u64) -> Option<TraceItem> {
    aio_workledger_types::get_trace(index)
}

#[ic_cdk::query]
fn get_trace_by_id(trace_id: String) -> Option<TraceItem> {
    aio_workledger_types::get_trace_by_id(trace_id)
}

#[ic_cdk::query]
fn get_user_traces() -> Vec<TraceItem> {
    let caller_id = caller().to_string();
    aio_workledger_types::get_user_traces(caller_id)
}

#[ic_cdk::query]
fn get_user_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    let caller_id = caller().to_string();
    aio_workledger_types::get_user_traces_paginated(caller_id, offset, limit)
}

#[ic_cdk::query]
fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    aio_workledger_types::get_traces_paginated(offset, limit)
}

#[ic_cdk::update]
fn add_trace(mut trace: TraceItem) -> Result<u64, String> {
    let caller_id = caller().to_string();
    trace.owner = caller_id;
    trace.created_at = ic_cdk::api::time() / 1_000_000; // Convert nanoseconds to milliseconds
    trace.updated_at = trace.created_at;
    
    // We no longer generate a trace_id - it must come from the frontend
    // as shown in the example: "trace_id": "AIO-TR-20250326-0001"
    
    aio_workledger_types::add_trace(trace)
}

// Helper function to generate current timestamp as string in YYYYMMDD format
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

