use candid::CandidType;
use ic_stable_structures::{StableBTreeMap, Storable, memory_manager::{MemoryId, MemoryManager, VirtualMemory}, DefaultMemoryImpl};
use serde::{Serialize, Deserialize as SerdeDeserialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::cell::RefCell;
use crate::stable_mem_storage::INVERTED_INDEX_STORE;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Public API functions
pub fn store_inverted_index(json_str: String) -> Result<(), String> {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow_mut().store_from_json(&json_str)
    })
}

pub fn get_all_inverted_index_items() -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().get_all_items()
    })
}

pub fn find_inverted_index_by_keyword(keyword: String) -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_keyword(&keyword)
    })
}

pub fn find_inverted_index_by_group(group: String) -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_keyword_group(&group)
    })
}

pub fn find_inverted_index_by_mcp(mcp_name: String) -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_mcp_name(&mcp_name)
    })
}

pub fn find_inverted_index_by_confidence(min_confidence: f32) -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_confidence(min_confidence)
    })
}

pub fn find_inverted_index_by_keywords(keywords: Vec<String>, min_confidence: f32) -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().find_by_keywords(&keywords, min_confidence)
    })
}

pub fn delete_inverted_index_by_mcp(mcp_name: String) -> Result<(), String> {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow_mut().delete_by_mcp_name(&mcp_name)
    })
}

pub fn get_all_keywords() -> String {
    INVERTED_INDEX_STORE.with(|store| {
        store.borrow().get_all_keywords()
    })
}

// add validate json str
pub fn validate_json_str(json_str: &str) -> Result<(), String> {
    let items: Vec<InvertedIndexItem> = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON format: {}", e))?;
        
    for item in items {
        if item.standard_match.is_empty() {
            return Err(format!("Invalid item: standard_match is empty for keyword {}", item.keyword));
        }
    }
    
    Ok(())
}

#[derive(CandidType, Clone, Debug, Serialize, SerdeDeserialize)]
pub struct InvertedIndexItem {
    pub keyword: String,
    pub keyword_group: String,
    pub mcp_name: String,
    pub method_name: String,
    pub source_field: String,
    pub confidence: f32,
    pub standard_match: String,
}

impl Storable for InvertedIndexItem {
    fn to_bytes(&self) -> Cow<[u8]> {
        match serde_json::to_vec(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(e) => {
                ic_cdk::println!("Error serializing InvertedIndexItem: {}", e);
                Cow::Owned(vec![])
            }
        }
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        if bytes.is_empty() {
            ic_cdk::println!("Warning: Attempting to deserialize empty bytes");
            return Self::default();
        }
        
        match serde_json::from_slice(&bytes) {
            Ok(item) => item,
            Err(e) => {
                ic_cdk::println!("Error deserializing InvertedIndexItem: {}", e);
                Self::default()
            }
        }
    }

    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Bounded {
        max_size: 10*1024,
        is_fixed_size: false,
    };
}

impl Default for InvertedIndexItem {
    fn default() -> Self {
        Self {
            keyword: String::new(),
            keyword_group: String::new(),
            mcp_name: String::new(),
            method_name: String::new(),
            source_field: String::new(),
            confidence: 0.0,
            standard_match: String::new(),
        }
    }
}

pub struct InvertedIndexStore {
    items: StableBTreeMap<Vec<u8>, InvertedIndexItem, Memory>,
    keyword_to_docs: HashMap<String, Vec<String>>,
}

impl InvertedIndexStore {
    pub fn new(memory: Memory) -> Self {
        Self {
            items: StableBTreeMap::new(memory),
            keyword_to_docs: HashMap::new(),
        }
    }

    // Get all unique keywords
    pub fn get_all_keywords(&self) -> String {
        let keywords: Vec<String> = self.keyword_to_docs.keys().cloned().collect();
        ic_cdk::println!("Retrieved {} unique keywords", keywords.len());
        serde_json::to_string(&keywords).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing keywords: {}", e);
            "[]".to_string()
        })
    }

    // Store inverted index from JSON string
    pub fn store_from_json(&mut self, json_str: &str) -> Result<(), String> {
        // add log
        ic_cdk::println!("Parsing JSON string: {}", json_str);
        
        let items: Vec<InvertedIndexItem> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // validate each item's standard_match field
        for item in &items {
            if item.standard_match.is_empty() {
                return Err("standard_match field cannot be empty".to_string());
            }
            ic_cdk::println!("Processing item - keyword: {}, standard_match: {}", 
                item.keyword, item.standard_match);
        }

        for mut item in items {
            // if method_name is help, then add help-for-mcp_name to keyword
            if item.method_name == "help" {
                item.keyword = format!("help-for-{}", item.mcp_name);
            }

            let key = format!("{}:{}:{}", item.keyword, item.mcp_name, item.standard_match).into_bytes();
            self.items.insert(key, item.clone());

            // Update keyword to document mapping
            self.keyword_to_docs
                .entry(item.keyword.clone())
                .or_insert_with(Vec::new)
                .push(item.mcp_name.clone());
        }

        Ok(())
    }

    // Get all inverted index items
    pub fn get_all_items(&self) -> String {
        let items: Vec<InvertedIndexItem> = self.items.iter().map(|(_, v)| v.clone()).collect();
        ic_cdk::println!("Retrieved {} items from storage", items.len());
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }
    

    // Find index items by keyword
    pub fn find_by_keyword(&self, keyword: &str) -> String {
        let keyword_parts: Vec<String> = keyword.split('-')
            .map(|s| s.to_lowercase())
            .collect();
        
        let mut items_with_matches: Vec<(InvertedIndexItem, usize)> = self.items
            .iter()
            .filter_map(|(k, v)| {
                let key_str = String::from_utf8_lossy(&k).to_lowercase();
                // Count how many parts of the keyword match in this item
                let match_count = keyword_parts.iter()
                    .filter(|part| key_str.contains(&part[..]))
                    .count();
                
                if match_count > 0 {
                    Some((v.clone(), match_count))
                } else {
                    None
                }
            })
            .collect();

        // Sort by number of matches in descending order
        items_with_matches.sort_by(|a, b| b.1.cmp(&a.1));

        let items: Vec<InvertedIndexItem> = items_with_matches.into_iter().map(|(item, _)| item).collect();
        ic_cdk::println!("Found {} items for keyword: {}", items.len(), keyword);
        ic_cdk::println!("Found Items: {:?}", items);
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }

    // Find index items by keyword group
    pub fn find_by_keyword_group(&self, group: &str) -> String {
        let items = self.items
            .iter()
            .filter(|(_, v)| v.keyword_group == group)
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();
        ic_cdk::println!("Found {} items for group: {}", items.len(), group);
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }

    // Find index items by MCP name
    pub fn find_by_mcp_name(&self, mcp_name: &str) -> String {
        let items = self.items
            .iter()
            .filter(|(k, _)| String::from_utf8_lossy(k).contains(&format!(":{}:", mcp_name)))
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();
        ic_cdk::println!("Found {} items for MCP: {}", items.len(), mcp_name);
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }

    // Find index items by confidence threshold
    pub fn find_by_confidence(&self, min_confidence: f32) -> String {
        let items = self.items
            .iter()
            .filter(|(_, v)| v.confidence >= min_confidence)
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();
        ic_cdk::println!("Found {} items with confidence >= {}", items.len(), min_confidence);
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }

    // Find index items by multiple keywords with confidence threshold
    pub fn find_by_keywords(&self, keywords: &[String], min_confidence: f32) -> String {
        let mut results: HashMap<String, (InvertedIndexItem, usize)> = HashMap::new();

        for keyword in keywords {
            let items = self.find_by_keyword(keyword);
            let items: Vec<InvertedIndexItem> = serde_json::from_str(&items).unwrap_or_default();
            for item in items {
                if item.confidence >= min_confidence {
                    let entry = results.entry(item.mcp_name.clone())
                        .or_insert_with(|| (item.clone(), 0));
                    entry.1 += 1; // Increment match count
                }
            }
        }

        // Convert to Vec and sort by match count and confidence
        let mut result_vec: Vec<(InvertedIndexItem, usize)> = results
            .into_iter()
            .map(|(_, (item, count))| (item, count))
            .collect();

        // Sort by match count (descending) and then by confidence (descending)
        result_vec.sort_by(|a, b| {
            let count_cmp = b.1.cmp(&a.1);
            if count_cmp == std::cmp::Ordering::Equal {
                b.0.confidence.partial_cmp(&a.0.confidence).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                count_cmp
            }
        });

        let items: Vec<InvertedIndexItem> = result_vec.into_iter().map(|(item, _)| item).collect();
        ic_cdk::println!("Found {} items matching keywords with confidence >= {}", items.len(), min_confidence);
        serde_json::to_string(&items).unwrap_or_else(|e| {
            ic_cdk::println!("Error serializing items: {}", e);
            "[]".to_string()
        })
    }

    // Delete all index items for a specific MCP
    pub fn delete_by_mcp_name(&mut self, mcp_name: &str) -> Result<(), String> {
        let items_to_delete: Vec<Vec<u8>> = self
            .items
            .iter()
            .filter(|(k, _)| String::from_utf8_lossy(k).ends_with(&format!(":{}", mcp_name)))
            .map(|(k, _)| k.clone())
            .collect();

        for key in items_to_delete {
            if let Some(item) = self.items.remove(&key) {
                // Update keyword to document mapping
                if let Some(docs) = self.keyword_to_docs.get_mut(&item.keyword) {
                    docs.retain(|doc| doc != mcp_name);
                    if docs.is_empty() {
                        self.keyword_to_docs.remove(&item.keyword);
                    }
                }
            }
        }

        Ok(())
    }

    // Find the most suitable index item by keywords with strategy
    pub fn find_by_keywords_strategy(&self, keywords: &[String]) -> Option<InvertedIndexItem> {
        if keywords.is_empty() {
            ic_cdk::println!("Warning: Empty keywords provided to find_by_keywords_strategy");
            return None;
        }

        ic_cdk::println!("Searching for keywords: {:?}", keywords);
        
        let mut results: HashMap<String, (InvertedIndexItem, usize)> = HashMap::new();

        // Step 1: Split input keywords into word sequences
        let input_word_sequences: Vec<Vec<String>> = keywords.iter()
            .map(|k| k.split(|c| c == '-' || c == '_')
                .map(|s| s.to_lowercase())
                .collect())
            .collect();

        ic_cdk::println!("Input word sequences: {:?}", input_word_sequences);

        // Step 2: Collect all matching items
        for keyword in &input_word_sequences {
            let keyword_str = keyword.join("-");
            ic_cdk::println!("Finding by keyword: {:?}", keyword_str);
            let items = self.find_by_keyword(&keyword_str);
            let items: Vec<InvertedIndexItem> = match serde_json::from_str(&items) {
                Ok(items) => items,
                Err(e) => {
                    ic_cdk::println!("Error parsing items for keyword {:?}: {}", keyword, e);
                    continue;
                }
            };
            
            ic_cdk::println!("Found {} items for keyword {:?}", items.len(), keyword);
            
            for item in items {
                // Skip items with method_name 'help'
                if item.method_name == "help" {
                    ic_cdk::println!("Skipping help item for keyword {:?}", keyword);
                    continue;
                }
                // Skip items with confidence < 0.7
                if item.confidence < 0.7 {
                    ic_cdk::println!("Skipping low confidence item ({} < 0.7) for keyword {:?}", item.confidence, keyword);
                    continue;
                }

                // Split stored keyword into word sequence
                let stored_word_sequence: Vec<String> = item.keyword
                    .split(|c| c == '-' || c == '_')
                    .map(|s| s.to_lowercase())
                    .collect();

                ic_cdk::println!("Comparing stored sequence {:?} with input sequences", stored_word_sequence);

                // Calculate match score for this item
                let mut match_score = 0;
                for input_sequence in &input_word_sequences {
                    let mut sequence_match_count = 0;
                    for input_word in input_sequence {
                        if stored_word_sequence.contains(input_word) {
                            sequence_match_count += 1;
                        }
                    }
                    
                    // Check if more than half of the words match
                    if sequence_match_count > input_sequence.len() / 2 {
                        match_score += 1;
                    }
                }

                if match_score > 0 {
                    ic_cdk::println!("Found match with score {} for item {:?}", match_score, item);
                    let entry = results.entry(item.mcp_name.clone())
                        .or_insert_with(|| (item.clone(), 0));
                    entry.1 += match_score;
                }
            }
        }

        // Return None if no matches found
        if results.is_empty() {
            ic_cdk::println!("No matches found for any keywords");
            return None;
        }

        // Convert to Vec and sort
        let mut result_vec: Vec<(InvertedIndexItem, usize)> = results
            .into_iter()
            .map(|(_, (item, count))| (item, count))
            .collect();

        // Sort by standard_match == 'true', then by match score and confidence
        result_vec.sort_by(|a, b| {
            // First check standard_match
            let a_is_true = a.0.standard_match == "true";
            let b_is_true = b.0.standard_match == "true";
            if a_is_true != b_is_true {
                return b_is_true.cmp(&a_is_true);
            }
            
            // Then sort by match score
            let score_cmp = b.1.cmp(&a.1);
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }
            
            // Finally sort by confidence
            b.0.confidence.partial_cmp(&a.0.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return the first (most matching) item
        let result = result_vec.first().map(|(item, _)| item.clone());
        ic_cdk::println!("Selected best match: {:?}", result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::memory_manager::MemoryManager;
    use ic_stable_structures::DefaultMemoryImpl;

    fn setup_test_store() -> InvertedIndexStore {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let memory = memory_manager.get(MemoryId::new(88));
        InvertedIndexStore::new(memory)
    }

    #[test]
    fn test_store_and_retrieve_with_standard_match() {
        let mut store = setup_test_store();
        
        // Create test data
        let test_item = InvertedIndexItem {
            keyword: "test".to_string(),
            keyword_group: "group1".to_string(),
            mcp_name: "mcp1".to_string(),
            source_field: "field1".to_string(),
            confidence: 0.95,
            standard_match: "exact".to_string(),
        };

        // Convert to JSON and store
        let json_str = serde_json::to_string(&vec![test_item.clone()]).unwrap();
        store.store_from_json(&json_str).unwrap();

        // Verify storage
        let all_items = store.get_all_items();
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&all_items).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].standard_match, "exact");
    }

    #[test]
    fn test_find_by_keyword_with_standard_match() {
        let mut store = setup_test_store();
        
        // Create test data
        let test_items = vec![
            InvertedIndexItem {
                keyword: "test".to_string(),
                keyword_group: "group1".to_string(),
                mcp_name: "mcp1".to_string(),
                source_field: "field1".to_string(),
                confidence: 0.95,
                standard_match: "exact".to_string(),
            },
            InvertedIndexItem {
                keyword: "test".to_string(),
                keyword_group: "group1".to_string(),
                mcp_name: "mcp2".to_string(),
                source_field: "field1".to_string(),
                confidence: 0.85,
                standard_match: "partial".to_string(),
            },
        ];

        // Store data
        let json_str = serde_json::to_string(&test_items).unwrap();
        store.store_from_json(&json_str).unwrap();

        // Verify query
        let result = store.find_by_keyword("test");
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&result).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item.standard_match == "exact"));
        assert!(items.iter().any(|item| item.standard_match == "partial"));
    }

    #[test]
    fn test_find_by_mcp_name_with_standard_match() {
        let mut store = setup_test_store();
        
        // Create test data
        let test_item = InvertedIndexItem {
            keyword: "test".to_string(),
            keyword_group: "group1".to_string(),
            mcp_name: "mcp1".to_string(),
            source_field: "field1".to_string(),
            confidence: 0.95,
            standard_match: "exact".to_string(),
        };

        // Store data
        let json_str = serde_json::to_string(&vec![test_item.clone()]).unwrap();
        store.store_from_json(&json_str).unwrap();

        // Verify query
        let result = store.find_by_mcp_name("mcp1");
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&result).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].standard_match, "exact");
    }

    #[test]
    fn test_validate_json_str() {
        // Test valid JSON
        let valid_json = r#"[{
            "keyword": "test",
            "keyword_group": "group1",
            "mcp_name": "mcp1",
            "source_field": "field1",
            "confidence": 0.95,
            "standard_match": "exact"
        }]"#;
        assert!(validate_json_str(valid_json).is_ok());

        // Test invalid JSON (missing standard_match)
        let invalid_json = r#"[{
            "keyword": "test",
            "keyword_group": "group1",
            "mcp_name": "mcp1",
            "source_field": "field1",
            "confidence": 0.95
        }]"#;
        assert!(validate_json_str(invalid_json).is_err());

        // Test invalid JSON (empty standard_match)
        let empty_standard_match_json = r#"[{
            "keyword": "test",
            "keyword_group": "group1",
            "mcp_name": "mcp1",
            "source_field": "field1",
            "confidence": 0.95,
            "standard_match": ""
        }]"#;
        assert!(validate_json_str(empty_standard_match_json).is_err());
    }
} 