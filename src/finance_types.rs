use candid::{CandidType, Deserialize, Nat, Principal};
use serde::Serialize;
use icrc_ledger_types::{icrc1::account::Account, icrc1::transfer::{TransferArg, TransferError, BlockIndex}};
use crate::account_storage::{upsert_account, get_account};
use crate::trace_storage::{upsert_trace, get_trace, get_owner_traces, get_owner_traces_paginated};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use num_traits::cast::ToPrimitive;
use crate::aio_workledger_types::{IOData, CallItem};

// Re-export NumTokens for public use
pub use icrc_ledger_types::icrc1::transfer::NumTokens;

// Constants for time calculations
const SECS_PER_MIN: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * SECS_PER_MIN;
const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;
const DAYS_PER_YEAR: u64 = 365;
const DAYS_PER_MONTH: [u64; 12] = [31,28,31,30,31,30,31,31,30,31,30,31];

/// Format a UNIX timestamp (seconds since epoch) into a string.
/// Supported formats: "YYYY-MM-DD", "YYYY-MM", "YYYY-MM-DD-HH", "YYYY", "%V" (ISO week), "%A" (weekday), "%B" (month name), "%H" (hour)
fn format_time(timestamp: u64, fmt: &str) -> String {
    // Basic calculation for UTC time
    // Note: This is a minimal implementation and may not handle all edge cases (e.g., leap years, DST)
    // For IC, this is usually sufficient for grouping by day/month/year/hour.
    let mut secs = timestamp;
    let mut year = 1970;
    let mut month = 1;
    let mut day = 1;
    let mut hour = 0;
    let mut min = 0;
    let mut sec = 0;
    // Calculate year
    let mut days = secs / SECS_PER_DAY;
    secs %= SECS_PER_DAY;
    while days >= if is_leap_year(year) { 366 } else { 365 } {
        days -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }
    // Calculate month
    let mut month_days = DAYS_PER_MONTH;
    if is_leap_year(year) { month_days[1] = 29; }
    for (i, &mdays) in month_days.iter().enumerate() {
        if days + 1 > mdays {
            days -= mdays;
            month += 1;
        } else {
            break;
        }
    }
    day += days as u32;
    hour = (secs / SECS_PER_HOUR) as u32;
    secs %= SECS_PER_HOUR;
    min = (secs / SECS_PER_MIN) as u32;
    sec = (secs % SECS_PER_MIN) as u32;
    match fmt {
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", year, month, day),
        "%Y-%m" => format!("{:04}-{:02}", year, month),
        "%Y" => format!("{:04}", year),
        "%Y-%m-%d-%H" => format!("{:04}-{:02}-{:02}-{:02}", year, month, day, hour),
        "%H" => format!("{:02}", hour),
        "%A" => weekday_name(year, month, day),
        "%B" => month_name(month),
        "%V" => iso_week_number(year, month, day).to_string(),
        _ => format!("{:04}-{:02}-{:02}", year, month, day),
    }
}
fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
fn month_name(month: u32) -> String {
    match month {
        1 => "January", 2 => "February", 3 => "March", 4 => "April", 5 => "May", 6 => "June",
        7 => "July", 8 => "August", 9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "Unknown"
    }.to_string()
}
fn weekday_name(year: u64, month: u32, day: u32) -> String {
    // Zeller's congruence
    let (y, m) = if month < 3 { (year - 1, month + 12) } else { (year, month) };
    let k = y % 100;
    let j = y / 100;
    let h = (day as u64 + (13 * (m as u64 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    match h {
        0 => "Saturday", 1 => "Sunday", 2 => "Monday", 3 => "Tuesday", 4 => "Wednesday", 5 => "Thursday", 6 => "Friday", _ => "Unknown"
    }.to_string()
}
fn iso_week_number(year: u64, month: u32, day: u32) -> u32 {
    // Simple ISO week calculation (not 100% accurate for all edge cases)
    let y = year as i32;
    let m = month as i32;
    let d = day as i32;
    let mut a = (14 - m) / 12;
    let y = y - a;
    let m = m + 12 * a - 2;
    let dow = (d + y + y/4 - y/100 + y/400 + (31*m)/12) % 7;
    let mut doy = d;
    for i in 1..m { doy += DAYS_PER_MONTH[(i-1) as usize] as i32; }
    ((doy - dow + 10) / 7) as u32
}

pub use crate::aio_workledger_types::TraceItem;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AccountInfo {
    pub principal_id: String,
    pub token_balance: NumTokens,
    pub symbol: String,
    pub stack_balance: NumTokens,
    pub credit_balance: NumTokens,
    pub unclaimed_balance: NumTokens,
    pub last_claim_time: u64,
    pub last_claim_amount: NumTokens,
    pub last_claim_timestamp: u64
}

impl AccountInfo {
    /// Create a new account
    pub fn new(principal_id: String, symbol: String) -> Self {
        Self {
            principal_id,
            token_balance: NumTokens::from(0u64),
            symbol,
            stack_balance: NumTokens::from(0u64),
            credit_balance: NumTokens::from(0u64),
            unclaimed_balance: NumTokens::from(0u64),
            last_claim_time: 0,
            last_claim_amount: NumTokens::from(0u64),
            last_claim_timestamp: 0
        }
    }

    /// Add a new account to storage
    pub fn add_account(principal_id: String, symbol: String) -> Result<AccountInfo, String> {
        let account = Self::new(principal_id, symbol);
        upsert_account(account)
    }

    /// Get account information
    pub fn get_account_info(principal_id: String) -> Option<Self> {
        get_account(principal_id)
    }

    /// Create a trace item for a transaction
    fn create_trace_item(&self, operation: &str, amount: NumTokens, to_account: Option<Account>, status: TransferStatus, error: Option<String>) -> Result<TraceItem, String> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let call = CallItem {
            id: current_time, // or another unique identifier
            protocol: "finance".to_string(),
            agent: self.principal_id.clone(),
            call_type: operation.to_string(),
            method: operation.to_string(),
            inputs: vec![IOData { data_type: "amount".to_string(), value: amount.0.to_string() }],
            outputs: vec![],
            status: format!("{:?}", status),
        };

        let trace = TraceItem {
            context_id: self.principal_id.clone(),
            trace_id: format!("{}-{}-{}", self.principal_id, operation, current_time),
            owner: self.principal_id.clone(),
            created_at: current_time,
            updated_at: current_time,
            calls: vec![call],
            metadata: error.map(|e| format!("error: {}", e)),
        };

        // Store the trace immediately
        upsert_trace(trace.clone())?;
        Ok(trace)
    }

    /// Handle transfer result and update trace item
    fn handle_transfer_result(&self, trace_item: &mut TraceItem, result: TransferResult) -> Result<(), String> {
        trace_item.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(call) = trace_item.calls.get_mut(0) {
            if result.success {
                call.status = "Completed".to_string();
                if let Some(height) = result.block_height {
                    trace_item.trace_id = format!("{}-{}", trace_item.trace_id, height);
                }
            } else {
                call.status = "Failed".to_string();
                if let Some(error) = result.error {
                    trace_item.metadata = Some(format!("error: {:?}", error));
                }
            }
        }

        // Update the trace in storage
        upsert_trace(trace_item.clone())
    }

    /// Get all traces for this account
    pub fn get_traces(&self) -> Vec<TraceItem> {
        get_owner_traces(self.principal_id.clone())
    }

    /// Get paginated traces for this account
    pub fn get_traces_paginated(&self, offset: u64, limit: usize) -> Vec<TraceItem> {
        get_owner_traces_paginated(self.principal_id.clone(), offset, limit)
    }

    /// Get account balance summary
    pub fn get_balance_summary(&self) -> (NumTokens, NumTokens, NumTokens, NumTokens) {
        (
            self.token_balance.clone(),
            self.stack_balance.clone(),
            self.credit_balance.clone(),
            self.unclaimed_balance.clone()
        )
    }

    /// Stack tokens to the account with trace
    pub fn stack_token(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        if self.token_balance < amount {
            return Err("Insufficient token balance".to_string());
        }

        self.token_balance = self.token_balance.clone() - amount.clone();
        self.stack_balance = self.stack_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("stack", amount, None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Unstack tokens from the account with trace
    pub fn unstack_token(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        if self.stack_balance < amount {
            return Err("Insufficient stack balance".to_string());
        }

        self.stack_balance = self.stack_balance.clone() - amount.clone();
        self.token_balance = self.token_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("unstack", amount, None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Add credit to the account with trace
    pub fn add_credit(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        self.credit_balance = self.credit_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("add_credit", amount, None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Use credit from the account with trace
    pub fn use_credit(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        if self.credit_balance < amount {
            return Err("Insufficient credit balance".to_string());
        }

        self.credit_balance = self.credit_balance.clone() - amount.clone();

        let trace_item = self.create_trace_item("use_credit", amount, None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Batch transfer tokens to multiple accounts with trace
    pub fn batch_transfer(&mut self, transfers: Vec<(Account, NumTokens)>) -> Result<(Self, Vec<TraceItem>, Vec<TransferResult>), String> {
        if transfers.is_empty() {
            return Err("No transfers specified".to_string());
        }

        let total_amount: NumTokens = transfers.iter()
            .map(|(_, amount)| amount.clone())
            .fold(NumTokens::from(0u64), |acc, x| acc + x);

        if self.token_balance < total_amount {
            return Err("Insufficient token balance".to_string());
        }

        let mut trace_items = Vec::new();
        let mut transfer_results = Vec::new();

        for (to_account, amount) in transfers {
            let (updated_account, trace_item, transfer_result) = self.transfer_tokens(to_account, amount)?;
            *self = updated_account;
            trace_items.push(trace_item);
            transfer_results.push(transfer_result);
        }

        Ok((self.clone(), trace_items, transfer_results))
    }

    /// Transfer tokens to another account with trace
    pub fn transfer_tokens(&mut self, to_account: Account, amount: NumTokens) -> Result<(Self, TraceItem, TransferResult), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        if self.token_balance < amount {
            return Err("Insufficient token balance".to_string());
        }

        self.token_balance = self.token_balance.clone() - amount.clone();

        let trace_item = self.create_trace_item("transfer", amount.clone(), Some(to_account.clone()), TransferStatus::Pending, None)?;
        let transfer_result = TransferResult {
            success: true,
            error: None,
            block_height: None
        };

        Ok((self.clone(), trace_item, transfer_result))
    }

    /// Add tokens to unclaimed balance with trace
    pub fn add_unclaimed_balance(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        self.unclaimed_balance = self.unclaimed_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("add_unclaimed", amount.clone(), None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Claim tokens from unclaimed balance with trace
    pub fn claim_token(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        if self.unclaimed_balance < amount {
            return Err("Insufficient unclaimed balance".to_string());
        }

        self.unclaimed_balance = self.unclaimed_balance.clone() - amount.clone();
        self.token_balance = self.token_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("claim", amount.clone(), None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Add tokens to token balance with trace
    pub fn add_token_balance(&mut self, amount: NumTokens) -> Result<(Self, TraceItem), String> {
        if amount <= NumTokens::from(0u64) {
            return Err("Amount must be greater than 0".to_string());
        }

        self.token_balance = self.token_balance.clone() + amount.clone();

        let trace_item = self.create_trace_item("add_token", amount.clone(), None, TransferStatus::Pending, None)?;
        Ok((self.clone(), trace_item))
    }

    /// Get transaction history with filters
    pub fn get_transaction_history(
        &self,
        operation_type: Option<String>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        status: Option<TransferStatus>
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| {
                let matches_operation = operation_type.as_ref()
                    .map(|op| trace.calls.get(0).map_or(false, |call| call.method == *op))
                    .unwrap_or(true);
                
                let matches_time = start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true);
                
                let matches_status = status.as_ref()
                    .map(|s| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", *s)))
                    .unwrap_or(true);

                matches_operation && matches_time && matches_status
            })
            .collect()
    }

    /// Get paginated transaction history with filters
    pub fn get_transaction_history_paginated(
        &self,
        offset: u64,
        limit: usize,
        operation_type: Option<String>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        status: Option<TransferStatus>,
        min_amount: Option<NumTokens>,
        max_amount: Option<NumTokens>
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| {
                let matches_operation = operation_type.as_ref()
                    .map(|op| trace.calls.get(0).map_or(false, |call| call.method == *op))
                    .unwrap_or(true);
                
                let matches_time = start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true);
                
                let matches_status = status.as_ref()
                    .map(|s| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", *s)))
                    .unwrap_or(true);

                let matches_amount = min_amount.as_ref()
                    .map(|min| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) >= min.clone())
                    .unwrap_or(true)
                    && max_amount.as_ref()
                    .map(|max| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) <= max.clone())
                    .unwrap_or(true);

                matches_operation && matches_time && matches_status && matches_amount
            })
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get paginated traces by operation type
    pub fn get_traces_by_operation_paginated(
        &self,
        operation: &str,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| trace.calls.get(0).map_or(false, |call| call.method == *operation))
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get paginated traces by status
    pub fn get_traces_by_status_paginated(
        &self,
        status: TransferStatus,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", status)))
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get paginated traces by time range
    pub fn get_traces_by_time_range_paginated(
        &self,
        start_time: u64,
        end_time: u64,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| trace.created_at >= start_time && trace.created_at <= end_time)
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get paginated traces by amount range
    pub fn get_traces_by_amount_range_paginated(
        &self,
        min_amount: NumTokens,
        max_amount: NumTokens,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| {
                let amount = NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));
                amount >= min_amount && amount <= max_amount
            })
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get paginated traces by recipient
    pub fn get_traces_by_recipient_paginated(
        &self,
        recipient: Account,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| trace.calls.get(0).map_or(false, |call| call.agent == recipient.owner.to_string()))
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get total count of traces matching filters
    pub fn get_trace_count(
        &self,
        operation_type: Option<String>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        status: Option<TransferStatus>,
        min_amount: Option<NumTokens>,
        max_amount: Option<NumTokens>
    ) -> u64 {
        self.get_traces().into_iter()
            .filter(|trace| {
                let matches_operation = operation_type.as_ref()
                    .map(|op| trace.calls.get(0).map_or(false, |call| call.method == *op))
                    .unwrap_or(true);
                
                let matches_time = start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true);
                
                let matches_status = status.as_ref()
                    .map(|s| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", *s)))
                    .unwrap_or(true);

                let matches_amount = min_amount.as_ref()
                    .map(|min| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) >= min.clone())
                    .unwrap_or(true)
                    && max_amount.as_ref()
                    .map(|max| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) <= max.clone())
                    .unwrap_or(true);

                matches_operation && matches_time && matches_status && matches_amount
            })
            .count() as u64
    }

    /// Get summary statistics for traces
    pub fn get_trace_statistics(
        &self,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> (u64, NumTokens, NumTokens, NumTokens) {
        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            });

        let mut total_count = 0;
        let mut total_amount = NumTokens::from(0u64);
        let mut success_amount = NumTokens::from(0u64);
        let mut failed_amount = NumTokens::from(0u64);

        for trace in traces {
            total_count += 1;
            let amount = NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));
            total_amount = total_amount.clone() + amount.clone();
            
            if trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)) {
                success_amount = success_amount.clone() + amount.clone();
            }
        }

        (total_count, total_amount, success_amount, failed_amount)
    }

    /// Get paginated traces with sorting
    pub fn get_traces_sorted_paginated(
        &self,
        offset: u64,
        limit: usize,
        sort_by: &str,
        sort_desc: bool
    ) -> Vec<TraceItem> {
        let mut traces = self.get_traces();
        
        match sort_by {
            "amount" => {
                traces.sort_by(|a, b| {
                    if sort_desc {
                        b.calls.get(0).map_or(0, |a_call| a_call.inputs[0].value.parse::<u64>().unwrap_or(0))
                            .cmp(&a.calls.get(0).map_or(0, |b_call| b_call.inputs[0].value.parse::<u64>().unwrap_or(0)))
                    } else {
                        a.calls.get(0).map_or(0, |a_call| a_call.inputs[0].value.parse::<u64>().unwrap_or(0))
                            .cmp(&b.calls.get(0).map_or(0, |b_call| b_call.inputs[0].value.parse::<u64>().unwrap_or(0)))
                    }
                });
            },
            "time" => {
                traces.sort_by(|a, b| {
                    if sort_desc {
                        b.created_at.cmp(&a.created_at)
                    } else {
                        a.created_at.cmp(&b.created_at)
                    }
                });
            },
            "status" => {
                traces.sort_by(|a, b| {
                    let a_status = a.calls.get(0).map_or(String::new(), |call| call.status.clone());
                    let b_status = b.calls.get(0).map_or(String::new(), |call| call.status.clone());
                    if sort_desc {
                        b_status.cmp(&a_status)
                    } else {
                        a_status.cmp(&b_status)
                    }
                });
            },
            _ => {
                // Default sort by time
                traces.sort_by(|a, b| {
                    if sort_desc {
                        b.created_at.cmp(&a.created_at)
                    } else {
                        a.created_at.cmp(&b.created_at)
                    }
                });
            }
        }

        traces.into_iter()
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get traces grouped by operation type
    pub fn get_traces_by_operation_type(&self) -> HashMap<String, Vec<TraceItem>> {
        let mut grouped = HashMap::new();
        
        for trace in self.get_traces() {
            let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
            grouped.entry(operation)
                .or_insert_with(Vec::new)
                .push(trace);
        }
        
        grouped
    }

    /// Get traces grouped by status
    pub fn get_traces_by_status(&self) -> HashMap<TransferStatus, Vec<TraceItem>> {
        let mut grouped = HashMap::new();
        
        for trace in self.get_traces() {
            let status = trace.calls.get(0)
                .map(|call| transfer_status_from_str(&call.status))
                .unwrap_or(TransferStatus::Pending);
            grouped.entry(status)
                .or_insert_with(Vec::new)
                .push(trace);
        }
        
        grouped
    }

    /// Get traces grouped by time period (daily, weekly, monthly)
    pub fn get_traces_by_time_period(&self, period: &str) -> HashMap<String, Vec<TraceItem>> {
        let mut grouped = HashMap::new();
        
        for trace in self.get_traces() {
            let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(trace.created_at);
            let key = match period {
                "daily" => format_time(trace.created_at, "%Y-%m-%d"),
                "weekly" => format!("{}-W{}", format_time(trace.created_at, "%Y"), format_time(trace.created_at, "%V")),
                "monthly" => format_time(trace.created_at, "%Y-%m"),
                _ => format_time(trace.created_at, "%Y-%m-%d"),
            };
            
            grouped.entry(key)
                .or_insert_with(Vec::new)
                .push(trace);
        }
        
        grouped
    }

    /// Get traces with custom filter function
    pub fn get_traces_with_filter<F>(
        &self,
        filter_fn: F,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem>
    where
        F: Fn(&TraceItem) -> bool
    {
        self.get_traces().into_iter()
            .filter(filter_fn)
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get traces with multiple statuses
    pub fn get_traces_by_statuses_paginated(
        &self,
        statuses: Vec<TransferStatus>,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| {
                let status = trace.calls.get(0)
                    .map(|call| transfer_status_from_str(&call.status))
                    .unwrap_or(TransferStatus::Pending);
                statuses.contains(&status)
            })
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get traces with error messages
    pub fn get_traces_with_errors_paginated(
        &self,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status.contains("error")))
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get traces by amount ranges
    pub fn get_traces_by_amount_ranges_paginated(
        &self,
        ranges: Vec<(NumTokens, NumTokens)>,
        offset: u64,
        limit: usize
    ) -> Vec<TraceItem> {
        self.get_traces().into_iter()
            .filter(|trace| {
                let amount = NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));
                ranges.iter().any(|(min, max)| amount >= *min && amount <= *max)
            })
            .skip(offset as usize)
            .take(limit)
            .collect()
    }

    /// Get traces with detailed statistics
    pub fn get_traces_statistics_detailed(
        &self,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> HashMap<String, (u64, NumTokens, NumTokens, NumTokens)> {
        let mut stats = HashMap::new();
        
        for trace in self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            }) {
            let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
            let amount = NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));
            
            let (count, total, success, failed) = stats.entry(operation)
                .or_insert((0, NumTokens::from(0u64), NumTokens::from(0u64), NumTokens::from(0u64)));
            
            *count += 1;
            *total = total.clone() + amount.clone();
            
            if trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)) {
                *success = success.clone() + amount.clone();
            }
        }
        
        stats
    }

    /// Get traces with trend analysis
    pub fn get_traces_trend_analysis(
        &self,
        period: &str,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> TrendAnalysis {
        let mut analysis = TrendAnalysis {
            amount_trend: BTreeMap::new(),
            count_trend: BTreeMap::new(),
            success_rate_trend: BTreeMap::new(),
            average_amount_trend: BTreeMap::new(),
            growth_rate: 0.0,
            volatility: 0.0,
        };

        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            });

        let mut period_data: HashMap<String, (u64, NumTokens, u64)> = HashMap::new();

        for trace in traces {
            let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(trace.created_at);
            let key = match period {
                "hour" => format_time(trace.created_at, "%Y-%m-%d-%H"),
                "day" => format_time(trace.created_at, "%Y-%m-%d"),
                "week" => format!("{}-W{}", format_time(trace.created_at, "%Y"), format_time(trace.created_at, "%V")),
                "month" => format_time(trace.created_at, "%Y-%m"),
                _ => format_time(trace.created_at, "%Y-%m-%d"),
            };

            let (count, amount, success_count) = period_data.entry(key.clone())
                .or_insert((0, NumTokens::from(0u64), 0));

            *count += 1;
            *amount = amount.clone() + NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));
            if trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)) {
                *success_count += 1;
            }
        }

        // Calculate trends
        for (key, (count, amount, success_count)) in period_data {
            let amount_clone = amount.clone();
            analysis.amount_trend.insert(key.clone(), amount);
            analysis.count_trend.insert(key.clone(), count);
            analysis.success_rate_trend.insert(key.clone(), 
                if count > 0 { (success_count as f64 / count as f64) * 100.0 } else { 0.0 });
            analysis.average_amount_trend.insert(key.clone(), 
                if count > 0 { amount_clone.0.to_f64().unwrap_or(0.0) / count as f64 } else { 0.0 });
        }

        // Calculate growth rate and volatility
        if analysis.amount_trend.len() >= 2 {
            let amounts: Vec<f64> = analysis.amount_trend.values()
                .map(|amount| amount.0.to_f64().unwrap_or(0.0))
                .collect();
            
            // Calculate growth rate
            let first_amount = amounts.first().unwrap();
            let last_amount = amounts.last().unwrap();
            analysis.growth_rate = if *first_amount > 0.0 {
                ((*last_amount - *first_amount) / *first_amount) * 100.0
            } else {
                0.0
            };

            // Calculate volatility (standard deviation)
            let mean = amounts.iter().sum::<f64>() / amounts.len() as f64;
            let variance = amounts.iter()
                .map(|amount| (amount - mean).powi(2))
                .sum::<f64>() / amounts.len() as f64;
            analysis.volatility = variance.sqrt();
        }

        analysis
    }

    /// Get traces with correlation analysis
    pub fn get_traces_correlation_analysis(
        &self,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> CorrelationAnalysis {
        let mut analysis = CorrelationAnalysis {
            amount_time_correlation: 0.0,
            amount_status_correlation: HashMap::new(),
            time_status_correlation: HashMap::new(),
            operation_patterns: HashMap::new(),
        };

        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if traces.is_empty() {
            return analysis;
        }

        // Calculate amount-time correlation
        let times: Vec<f64> = traces.iter()
            .map(|trace| trace.created_at as f64)
            .collect();
        let amounts: Vec<f64> = traces.iter()
            .map(|trace| trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0)))
            .collect();
        analysis.amount_time_correlation = calculate_correlation(&times, &amounts);

        // Calculate amount-status correlation
        for status in [TransferStatus::Completed, TransferStatus::Failed, TransferStatus::Pending] {
            let status_amounts: Vec<f64> = traces.iter()
                .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", status)) &&
                    trace.calls.get(0).map_or(false, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0) > 0.0))
                .map(|trace| trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0)))
                .collect();
            
            if !status_amounts.is_empty() {
                let avg_amount = status_amounts.iter().sum::<f64>() / status_amounts.len() as f64;
                analysis.amount_status_correlation.insert(status, avg_amount);
            }
        }

        // Calculate time-status correlation
        for status in [TransferStatus::Completed, TransferStatus::Failed, TransferStatus::Pending] {
            let status_times: Vec<f64> = traces.iter()
                .filter(|trace| trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", status)) &&
                    trace.calls.get(0).map_or(false, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0) > 0.0))
                .map(|trace| trace.created_at as f64)
                .collect();
            
            if !status_times.is_empty() {
                let avg_time = status_times.iter().sum::<f64>() / status_times.len() as f64;
                analysis.time_status_correlation.insert(status, avg_time);
            }
        }

        // Analyze operation patterns
        for trace in &traces {
            let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
            let pattern = analysis.operation_patterns
                .entry(operation)
                .or_insert_with(|| OperationPattern {
                    total_count: 0,
                    success_count: 0,
                    total_amount: NumTokens::from(0u64),
                    average_amount: 0.0,
                    time_distribution: HashMap::new(),
                });

            pattern.total_count += 1;
            if trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)) {
                pattern.success_count += 1;
            }
            pattern.total_amount = pattern.total_amount.clone() + NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64)));

            let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(trace.created_at);
            let hour = format_time(trace.created_at, "%H");
            *pattern.time_distribution.entry(hour).or_insert(0) += 1;
        }

        // Calculate average amounts for each operation pattern
        for pattern in analysis.operation_patterns.values_mut() {
            pattern.average_amount = if pattern.total_count > 0 {
                pattern.total_amount.0.to_f64().unwrap_or(0.0) / pattern.total_count as f64
            } else {
                0.0
            };
        }

        analysis
    }

    /// Get traces with anomaly detection
    pub fn get_traces_anomaly_detection(
        &self,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> AnomalyDetection {
        let mut detection = AnomalyDetection {
            amount_anomalies: Vec::new(),
            time_anomalies: Vec::new(),
            status_anomalies: Vec::new(),
            pattern_anomalies: Vec::new(),
        };

        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if traces.is_empty() {
            return detection;
        }

        // Calculate amount statistics
        let amounts: Vec<f64> = traces.iter()
            .map(|trace| trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0)))
            .collect();
        let mean_amount = amounts.iter().sum::<f64>() / amounts.len() as f64;
        let std_amount = calculate_standard_deviation(&amounts);

        // Detect amount anomalies
        for trace in &traces {
            let amount = trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0));
            if (amount - mean_amount).abs() > 2.0 * std_amount {
                detection.amount_anomalies.push(trace.clone());
            }
        }

        // Calculate time statistics
        let times: Vec<f64> = traces.iter()
            .map(|trace| trace.created_at as f64)
            .collect();
        let mean_time = times.iter().sum::<f64>() / times.len() as f64;
        let std_time = calculate_standard_deviation(&times);

        // Detect time anomalies
        for trace in &traces {
            let time = trace.created_at as f64;
            if (time - mean_time).abs() > 2.0 * std_time {
                detection.time_anomalies.push(trace.clone());
            }
        }

        // Detect status anomalies
        let status_counts: HashMap<TransferStatus, usize> = traces.iter()
            .fold(HashMap::new(), |mut acc, trace| {
                *acc.entry(trace.calls.get(0).map_or(TransferStatus::Pending, |call| {
                    transfer_status_from_str(&call.status)
                }))
                    .or_insert(0) += 1;
                acc
            });

        let total_traces = traces.len();
        for trace in &traces {
            let status_count = status_counts.get(&trace.calls.get(0).map_or(TransferStatus::Pending, |call| {
                transfer_status_from_str(&call.status)
            })).unwrap_or(&0);
            let status_ratio = *status_count as f64 / total_traces as f64;
            if status_ratio < 0.1 { // Less than 10% of total
                detection.status_anomalies.push(trace.clone());
            }
        }

        // Detect pattern anomalies
        let operation_counts: HashMap<String, usize> = traces.iter()
            .fold(HashMap::new(), |mut acc, trace| {
                let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
                *acc.entry(operation).or_insert(0) += 1;
                acc
            });

        for trace in &traces {
            let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
            let operation_count = operation_counts.get(&operation).unwrap_or(&0);
            let operation_ratio = *operation_count as f64 / total_traces as f64;
            if operation_ratio < 0.05 { // Less than 5% of total
                detection.pattern_anomalies.push(trace.clone());
            }
        }

        detection
    }

    /// Get traces with predictive analysis
    pub fn get_traces_predictive_analysis(
        &self,
        window_size: usize,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> PredictiveAnalysis {
        let mut analysis = PredictiveAnalysis {
            amount_forecast: Vec::new(),
            count_forecast: Vec::new(),
            success_rate_forecast: Vec::new(),
            seasonal_patterns: HashMap::new(),
            moving_averages: MovingAverages {
                amount: VecDeque::new(),
                count: VecDeque::new(),
                success_rate: VecDeque::new(),
            },
            confidence_intervals: HashMap::new(),
        };

        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if traces.is_empty() {
            return analysis;
        }

        // Calculate moving averages
        let mut amount_window = VecDeque::with_capacity(window_size);
        let mut count_window = VecDeque::with_capacity(window_size);
        let mut success_window = VecDeque::with_capacity(window_size);

        for trace in &traces {
            amount_window.push_back(trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0)));
            count_window.push_back(1.0);
            success_window.push_back(if trace.calls.get(0).map_or(false, |call| call.status == format!("{:?}", TransferStatus::Completed)) { 1.0 } else { 0.0 });

            if amount_window.len() > window_size {
                amount_window.pop_front();
                count_window.pop_front();
                success_window.pop_front();
            }

            if amount_window.len() == window_size {
                analysis.moving_averages.amount.push_back(
                    amount_window.iter().sum::<f64>() / window_size as f64
                );
                analysis.moving_averages.count.push_back(
                    count_window.iter().sum::<f64>() / window_size as f64
                );
                analysis.moving_averages.success_rate.push_back(
                    success_window.iter().sum::<f64>() / window_size as f64
                );
            }
        }

        // Calculate seasonal patterns
        for trace in &traces {
            let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(trace.created_at);
            let hour = format_time(trace.created_at, "%H");
            let day = format_time(trace.created_at, "%A");
            let month = format_time(trace.created_at, "%B");

            let patterns = analysis.seasonal_patterns
                .entry("hourly".to_string())
                .or_insert_with(HashMap::new);
            *patterns.entry(hour).or_insert(0) += 1;

            let patterns = analysis.seasonal_patterns
                .entry("daily".to_string())
                .or_insert_with(HashMap::new);
            *patterns.entry(day).or_insert(0) += 1;

            let patterns = analysis.seasonal_patterns
                .entry("monthly".to_string())
                .or_insert_with(HashMap::new);
            *patterns.entry(month).or_insert(0) += 1;
        }

        // Generate forecasts
        if !analysis.moving_averages.amount.is_empty() {
            let last_amount = analysis.moving_averages.amount.back().unwrap();
            let last_count = analysis.moving_averages.count.back().unwrap();
            let last_success_rate = analysis.moving_averages.success_rate.back().unwrap();

            // Simple linear forecast for next 5 periods
            for i in 1..=5 {
                let trend_factor = 1.0 + (i as f64 * 0.1); // 10% growth assumption
                analysis.amount_forecast.push(last_amount * trend_factor);
                analysis.count_forecast.push(last_count * trend_factor);
                analysis.success_rate_forecast.push(*last_success_rate);
            }

            // Calculate confidence intervals
            let amount_std = calculate_standard_deviation(&analysis.moving_averages.amount.iter().copied().collect::<Vec<f64>>());
            let count_std = calculate_standard_deviation(&analysis.moving_averages.count.iter().copied().collect::<Vec<f64>>());
            let success_std = calculate_standard_deviation(&analysis.moving_averages.success_rate.iter().copied().collect::<Vec<f64>>());

            analysis.confidence_intervals.insert("amount".to_string(), (amount_std * 1.96, amount_std * 1.96));
            analysis.confidence_intervals.insert("count".to_string(), (count_std * 1.96, count_std * 1.96));
            analysis.confidence_intervals.insert("success_rate".to_string(), (success_std * 1.96, success_std * 1.96));
        }

        analysis
    }

    /// Get traces with risk analysis
    pub fn get_traces_risk_analysis(
        &self,
        start_time: Option<u64>,
        end_time: Option<u64>
    ) -> RiskAnalysis {
        let mut analysis = RiskAnalysis {
            risk_score: 0.0,
            risk_factors: Vec::new(),
            risk_level: RiskLevel::Low,
            suspicious_patterns: Vec::new(),
            risk_metrics: RiskMetrics {
                amount_risk: 0.0,
                frequency_risk: 0.0,
                status_risk: 0.0,
                pattern_risk: 0.0,
            },
        };

        let traces = self.get_traces().into_iter()
            .filter(|trace| {
                start_time.map(|start| trace.created_at >= start).unwrap_or(true)
                    && end_time.map(|end| trace.created_at <= end).unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if traces.is_empty() {
            return analysis;
        }

        // Calculate amount risk
        let amounts: Vec<f64> = traces.iter()
            .map(|trace| trace.calls.get(0).map_or(0.0, |call| call.inputs[0].value.parse::<f64>().unwrap_or(0.0)))
            .collect();
        let mean_amount = amounts.iter().sum::<f64>() / amounts.len() as f64;
        let std_amount = calculate_standard_deviation(&amounts);
        
        for amount in amounts {
            let z_score = (amount - mean_amount) / std_amount;
            if z_score.abs() > 2.0 {
                analysis.risk_metrics.amount_risk += 0.25;
                analysis.risk_factors.push(RiskFactor {
                    factor_type: "amount".to_string(),
                    description: format!("Unusual amount: {}", amount),
                    severity: if z_score.abs() > 3.0 { 0.8 } else { 0.5 },
                });
            }
        }

        // Calculate frequency risk
        let mut time_gaps = Vec::new();
        let mut sorted_times: Vec<u64> = traces.iter()
            .map(|trace| trace.created_at)
            .collect();
        sorted_times.sort();

        for i in 1..sorted_times.len() {
            time_gaps.push((sorted_times[i] - sorted_times[i-1]) as f64);
        }

        if !time_gaps.is_empty() {
            let mean_gap = time_gaps.iter().sum::<f64>() / time_gaps.len() as f64;
            let std_gap = calculate_standard_deviation(&time_gaps);

            for gap in time_gaps {
                let z_score = (gap - mean_gap) / std_gap;
                if z_score.abs() > 2.0 {
                    analysis.risk_metrics.frequency_risk += 0.25;
                    analysis.risk_factors.push(RiskFactor {
                        factor_type: "frequency".to_string(),
                        description: format!("Unusual time gap: {} seconds", gap),
                        severity: if z_score.abs() > 3.0 { 0.8 } else { 0.5 },
                    });
                }
            }
        }

        // Calculate status risk
        let status_counts: HashMap<TransferStatus, usize> = traces.iter()
            .fold(HashMap::new(), |mut acc, trace| {
                *acc.entry(trace.calls.get(0).map_or(TransferStatus::Pending, |call| {
                    transfer_status_from_str(&call.status)
                }))
                    .or_insert(0) += 1;
                acc
            });

        let total_traces = traces.len();
        for (status, count) in status_counts {
            let ratio = count as f64 / total_traces as f64;
            if status == TransferStatus::Failed && ratio > 0.1 {
                analysis.risk_metrics.status_risk += 0.25;
                analysis.risk_factors.push(RiskFactor {
                    factor_type: "status".to_string(),
                    description: format!("High failure rate: {:.1}%", ratio * 100.0),
                    severity: ratio,
                });
            }
        }

        // Calculate pattern risk
        let operation_counts: HashMap<String, usize> = traces.iter()
            .fold(HashMap::new(), |mut acc, trace| {
                let operation = trace.calls.get(0).map_or("unknown".to_string(), |call| call.method.clone());
                *acc.entry(operation).or_insert(0) += 1;
                acc
            });

        for (operation, count) in operation_counts {
            let ratio = count as f64 / total_traces as f64;
            if ratio < 0.05 {
                analysis.risk_metrics.pattern_risk += 0.25;
                analysis.risk_factors.push(RiskFactor {
                    factor_type: "pattern".to_string(),
                    description: format!("Rare operation: {} ({:.1}%)", operation, ratio * 100.0),
                    severity: 1.0 - ratio,
                });
            }
        }

        // Calculate overall risk score
        analysis.risk_score = (
            analysis.risk_metrics.amount_risk +
            analysis.risk_metrics.frequency_risk +
            analysis.risk_metrics.status_risk +
            analysis.risk_metrics.pattern_risk
        ) / 4.0;

        // Determine risk level
        analysis.risk_level = if analysis.risk_score >= 0.8 {
            RiskLevel::High
        } else if analysis.risk_score >= 0.5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // Identify suspicious patterns
        for factor in &analysis.risk_factors {
            if factor.severity >= 0.8 {
                analysis.suspicious_patterns.push(factor.clone());
            }
        }

        analysis
    }
}

fn calculate_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.is_empty() {
        return 0.0;
    }

    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();
    let sum_y2: f64 = y.iter().map(|a| a * a).sum();

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn calculate_standard_deviation(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    
    variance.sqrt()
}

#[derive(Debug)]
pub struct TrendAnalysis {
    pub amount_trend: BTreeMap<String, NumTokens>,
    pub count_trend: BTreeMap<String, u64>,
    pub success_rate_trend: BTreeMap<String, f64>,
    pub average_amount_trend: BTreeMap<String, f64>,
    pub growth_rate: f64,
    pub volatility: f64,
}

#[derive(Debug)]
pub struct CorrelationAnalysis {
    pub amount_time_correlation: f64,
    pub amount_status_correlation: HashMap<TransferStatus, f64>,
    pub time_status_correlation: HashMap<TransferStatus, f64>,
    pub operation_patterns: HashMap<String, OperationPattern>,
}

#[derive(Debug)]
pub struct OperationPattern {
    pub total_count: u64,
    pub success_count: u64,
    pub total_amount: NumTokens,
    pub average_amount: f64,
    pub time_distribution: HashMap<String, u64>,
}

#[derive(Debug)]
pub struct AnomalyDetection {
    pub amount_anomalies: Vec<TraceItem>,
    pub time_anomalies: Vec<TraceItem>,
    pub status_anomalies: Vec<TraceItem>,
    pub pattern_anomalies: Vec<TraceItem>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub severity: f64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TransferResult {
    pub success: bool,
    pub error: Option<TransferError>,
    pub block_height: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TraceFilters {
    pub operation_types: Option<Vec<String>>,
    pub statuses: Option<Vec<TransferStatus>>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub min_amount: Option<NumTokens>,
    pub max_amount: Option<NumTokens>,
    pub recipients: Option<Vec<Account>>,
}

impl TraceFilters {
    pub fn matches(&self, trace: &TraceItem) -> bool {
        let matches_operation = self.operation_types.as_ref()
            .map(|ops| ops.iter().any(|op| trace.calls.get(0).map_or(false, |call| call.method == *op)))
            .unwrap_or(true);

        let matches_status = self.statuses.as_ref()
            .map(|statuses| statuses.contains(&trace.calls.get(0).map_or(TransferStatus::Pending, |call| {
                transfer_status_from_str(&call.status)
            })))
            .unwrap_or(true);

        let matches_time = self.start_time.map(|start| trace.created_at >= start).unwrap_or(true)
            && self.end_time.map(|end| trace.created_at <= end).unwrap_or(true);

        let matches_amount = self.min_amount.as_ref()
            .map(|min| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) >= min.clone())
            .unwrap_or(true)
            && self.max_amount.as_ref()
            .map(|max| NumTokens::from(trace.calls.get(0).map_or(0u64, |call| call.inputs[0].value.parse::<u64>().unwrap_or(0u64))) <= max.clone())
            .unwrap_or(true);

        let matches_recipient = self.recipients.as_ref()
            .map(|recipients| recipients.contains(&trace.calls.get(0).map_or(Account { owner: Principal::anonymous(), subaccount: None }, |call| {
                Account {
                    owner: Principal::from_text(&call.agent).unwrap_or(Principal::anonymous()),
                    subaccount: None,
                }
            })))
            .unwrap_or(true);

        matches_operation && matches_status && matches_time && matches_amount && matches_recipient
    }
}

#[derive(Debug, Clone)]
pub struct TraceSort {
    pub field: String,
    pub descending: bool,
}

#[derive(Debug, Clone)]
pub struct AmountAnalysis {
    pub total_count: u64,
    pub total_amount: NumTokens,
    pub min_amount: NumTokens,
    pub max_amount: NumTokens,
    pub average_amount: f64,
    pub amount_distribution: HashMap<u64, u64>,
    pub status_distribution: HashMap<TransferStatus, u64>,
}

#[derive(Debug, Clone)]
pub struct TimePatternAnalysis {
    pub hourly_distribution: HashMap<String, u64>,
    pub daily_distribution: HashMap<String, u64>,
    pub weekly_distribution: HashMap<String, u64>,
    pub monthly_distribution: HashMap<String, u64>,
    pub peak_hours: Vec<String>,
    pub peak_days: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RecipientAnalysis {
    pub recipient_counts: HashMap<String, u64>,
    pub recipient_amounts: HashMap<String, NumTokens>,
    pub top_recipients: Vec<String>,
    pub unique_recipients: usize,
}

#[derive(Debug, Clone)]
pub struct PredictiveAnalysis {
    pub amount_forecast: Vec<f64>,
    pub count_forecast: Vec<f64>,
    pub success_rate_forecast: Vec<f64>,
    pub seasonal_patterns: HashMap<String, HashMap<String, u64>>,
    pub moving_averages: MovingAverages,
    pub confidence_intervals: HashMap<String, (f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct MovingAverages {
    pub amount: VecDeque<f64>,
    pub count: VecDeque<f64>,
    pub success_rate: VecDeque<f64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub enum TransferStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RiskMetrics {
    pub amount_risk: f64,
    pub frequency_risk: f64,
    pub status_risk: f64,
    pub pattern_risk: f64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RiskAnalysis {
    pub risk_score: f64,
    pub risk_factors: Vec<RiskFactor>,
    pub risk_level: RiskLevel,
    pub suspicious_patterns: Vec<RiskFactor>,
    pub risk_metrics: RiskMetrics,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

fn transfer_status_from_str(s: &str) -> TransferStatus {
    match s {
        "Pending" => TransferStatus::Pending,
        "Completed" => TransferStatus::Completed,
        "Failed" => TransferStatus::Failed,
        _ => TransferStatus::Pending,
    }
}