use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use crate::finance_types::TraceItem;
use std::collections::HashMap;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Define the key for trace data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceKey {
    pub id: String,
}

impl ic_stable_structures::Storable for TraceKey {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.id).expect("Failed to encode TraceKey"))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let id = Decode!(bytes.as_ref(), String).expect("Failed to decode TraceKey");
        Self { id }
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static TRACES: RefCell<StableBTreeMap<TraceKey, TraceItem, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(11)))
        )
    );
}

/// Add or update a trace item
pub fn upsert_trace(trace: TraceItem) -> Result<(), String> {
    TRACES.with(|traces| {
        let mut traces = traces.borrow_mut();
        let key = TraceKey { id: trace.trace_id.clone() };
        traces.insert(key, trace);
        Ok(())
    })
}

/// Get a trace item by ID
pub fn get_trace(id: String) -> Option<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        let key = TraceKey { id };
        traces.get(&key).map(|trace| trace.clone())
    })
}

/// Get all traces for a specific owner
pub fn get_owner_traces(owner: String) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner)
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get all traces
pub fn get_all_traces() -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter().map(|(_, trace)| trace).collect()
    })
}

/// Get traces with pagination
pub fn get_traces_paginated(offset: u64, limit: usize) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces
            .iter()
            .skip(offset as usize)
            .take(limit)
            .map(|(_, trace)| trace)
            .collect()
    })
}

/// Get owner traces with pagination
pub fn get_owner_traces_paginated(owner: String, offset: u64, limit: usize) -> Vec<TraceItem> {
    TRACES.with(|traces| {
        let traces = traces.borrow();
        traces.iter()
            .filter(|(_, trace)| trace.owner == owner)
            .map(|(_, trace)| trace)
            .skip(offset as usize)
            .take(limit)
            .collect()
    })
}

/// Delete a trace
pub fn delete_trace(id: String) -> Result<(), String> {
    TRACES.with(|traces| {
        let mut traces = traces.borrow_mut();
        let key = TraceKey { id };
        if traces.remove(&key).is_some() {
            Ok(())
        } else {
            Err("Trace not found".to_string())
        }
    })
} 