use candid::{CandidType, Decode, Encode}; // Remove unused Principal import
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Trace data structure for workflow ledger
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TraceItem {
    pub context_id: String,
    pub trace_id: String,
    pub owner: String, // Principal ID as string
    pub created_at: u64,
    pub updated_at: u64,
    pub calls: Vec<CallItem>,
    pub metadata: Option<String>, // Additional metadata as JSON
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CallItem {
    pub id: u64,
    pub protocol: String,
    pub agent: String,
    pub call_type: String, // renamed from 'type' which is a reserved keyword
    pub method: String,
    pub inputs: Vec<IOData>, // Changed from input to inputs (array)
    pub outputs: Vec<IOData>, // Changed from output to outputs (array)
    pub status: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct IOData {
    pub data_type: String, // renamed from 'type' which is a reserved keyword
    pub value: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TraceStatus {
    Ok,
    Recall,
    Fail
}

impl ic_stable_structures::Storable for TraceItem {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 10 * 1024 * 1024, is_fixed_size: false }; // 10MB should be sufficient for complex traces
}

// Define the key for user data association
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserTraceKey {
    pub owner: String,
    pub trace_id: String,
}

impl ic_stable_structures::Storable for UserTraceKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.owner, &self.trace_id).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let (owner, trace_id) = Decode!(bytes.as_ref(), String, String).unwrap();
        Self { owner, trace_id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 2048, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static TRACE_ITEMS: RefCell<StableVec<TraceItem, Memory>> = RefCell::new(
        StableVec::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
        ).unwrap()
    );

    static USER_TRACE_INDEX: RefCell<StableBTreeMap<UserTraceKey, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
        )
    );

    // Index for looking up traces by trace_id
    static TRACE_ID_INDEX: RefCell<StableBTreeMap<String, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6)))
        )
    );
}

/// Add a new trace to the storage
pub fn add_trace(trace: TraceItem) -> Result<u64, String> {
    TRACE_ITEMS.with(|items| {
        let mut items = items.borrow_mut();
        let index = items.len();
        let mut new_trace = trace;
        
        // Validate that trace_id is provided by the frontend
        if new_trace.trace_id.is_empty() {
            return Err("Trace ID must be provided".to_string());
        }
        
        // Check if trace_id already exists
        if TRACE_ID_INDEX.with(|idx| idx.borrow().contains_key(&new_trace.trace_id)) {
            return Err(format!("Trace with ID '{}' already exists", new_trace.trace_id));
        }
        
        // Store the trace
        items.push(&new_trace).map_err(|e| format!("Failed to store trace: {:?}", e))?;
        
        // Create owner index entry
        USER_TRACE_INDEX.with(|index_map| {
            let mut index_map = index_map.borrow_mut();
            let key = UserTraceKey { 
                owner: new_trace.owner.clone(), 
                trace_id: new_trace.trace_id.clone(),
            };
            index_map.insert(key, index);
        });
        
        // Create trace_id index entry
        TRACE_ID_INDEX.with(|index_map| {
            let mut index_map = index_map.borrow_mut();
            index_map.insert(new_trace.trace_id.clone(), index);
        });
        
        Ok(index)
    })
}

/// Get a trace by index
pub fn get_trace(index: u64) -> Option<TraceItem> {
    TRACE_ITEMS.with(|items| {
        let items = items.borrow();
        if index < items.len() {
            items.get(index)
        } else {
            None
        }
    })
}

/// Get a trace by trace_id
pub fn get_trace_by_id(trace_id: String) -> Option<TraceItem> {
    TRACE_ID_INDEX.with(|index| {
        let index = index.borrow();
        if let Some(item_index) = index.get(&trace_id) {
            get_trace(item_index)
        } else {
            None
        }
    })
}

/// Get all traces for a specific user
pub fn get_user_traces(owner: String) -> Vec<TraceItem> {
    let mut result = Vec::new();
    
    USER_TRACE_INDEX.with(|index| {
        let index = index.borrow();
        
        // Create range bounds for this user
        let start_key = UserTraceKey { 
            owner: owner.clone(), 
            trace_id: String::new() 
        };
        let end_key = UserTraceKey { 
            owner: owner.clone(), 
            trace_id: String::from_utf8(vec![0xFF; 100]).unwrap_or_default()
        };
        
        // Get all traces in range
        for (_, item_id) in index.range(start_key..=end_key) {
            if let Some(trace) = get_trace(item_id) {
                result.push(trace);
            }
        }
    });
    
    result
}

/// Get user traces with pagination
pub fn get_user_traces_paginated(owner: String, offset: u64, limit: usize) -> Vec<TraceItem> {
    let user_traces = get_user_traces(owner);
    
    if offset >= user_traces.len() as u64 {
        return Vec::new();
    }
    
    let end = std::cmp::min(offset as usize + limit, user_traces.len());
    user_traces[offset as usize..end].to_vec()
}

/// Get all traces with pagination
pub fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    TRACE_ITEMS.with(|items| {
        let items = items.borrow();
        let total_items = items.len();
        
        if offset >= total_items {
            return Vec::new();
        }
        
        let end = std::cmp::min(offset + limit as u64, total_items);
        let mut result = Vec::new();
        
        for i in offset..end {
            result.push(items.get(i).unwrap());
        }
        
        result
    })
}
