use candid::CandidType;
use ic_stable_structures::{StableBTreeMap, Storable, memory_manager::{MemoryId, MemoryManager, VirtualMemory}, DefaultMemoryImpl};
use serde::{Serialize, Deserialize as SerdeDeserialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::cell::RefCell;

type Memory = VirtualMemory<DefaultMemoryImpl>;

// Define memory manager
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
}

// Define inverted index store
thread_local! {
    pub static INVERTED_INDEX_STORE: RefCell<InvertedIndexStore> = RefCell::new(
        InvertedIndexStore::new(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))))
    );
}

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

// 添加验证 JSON 字符串的公共方法
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
        let bytes = serde_json::to_vec(self).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_json::from_slice(&bytes).unwrap()
    }

    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Bounded {
        max_size: 1024,
        is_fixed_size: false,
    };
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
        // 添加日志记录
        ic_cdk::println!("Parsing JSON string: {}", json_str);
        
        let items: Vec<InvertedIndexItem> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // 验证每个项目的 standard_match 字段
        for item in &items {
            if item.standard_match.is_empty() {
                return Err("standard_match field cannot be empty".to_string());
            }
            ic_cdk::println!("Processing item - keyword: {}, standard_match: {}", 
                item.keyword, item.standard_match);
        }

        for item in items {
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
        let items = self.items
            .iter()
            .filter(|(k, _)| String::from_utf8_lossy(k).starts_with(&format!("{}:", keyword)))
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>();
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
        let mut results: HashMap<String, (InvertedIndexItem, usize)> = HashMap::new();

        // Step 1: Collect all matching items
        for keyword in keywords {
            let items = self.find_by_keyword(keyword);
            let items: Vec<InvertedIndexItem> = serde_json::from_str(&items).unwrap_or_default();
            for item in items {
                // Skip items with method_name 'help'
                if item.method_name == "help" {
                    continue;
                }
                // Skip items with confidence < 0.9
                if item.confidence < 0.9 {
                    continue;
                }
                let entry = results.entry(item.mcp_name.clone())
                    .or_insert_with(|| (item.clone(), 0));
                entry.1 += 1; // Increment match count
            }
        }

        // Return None if no matches found
        if results.is_empty() {
            return None;
        }

        // Convert to Vec and sort
        let mut result_vec: Vec<(InvertedIndexItem, usize)> = results
            .into_iter()
            .map(|(_, (item, count))| (item, count))
            .collect();

        // Sort by standard_match == 'true', then by match count and confidence
        result_vec.sort_by(|a, b| {
            // First check standard_match
            let a_is_true = a.0.standard_match == "true";
            let b_is_true = b.0.standard_match == "true";
            if a_is_true != b_is_true {
                return b_is_true.cmp(&a_is_true);
            }
            
            // Then sort by match count
            let count_cmp = b.1.cmp(&a.1);
            if count_cmp != std::cmp::Ordering::Equal {
                return count_cmp;
            }
            
            // Finally sort by confidence
            b.0.confidence.partial_cmp(&a.0.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return the first (most matching) item
        result_vec.first().map(|(item, _)| item.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::memory_manager::MemoryManager;
    use ic_stable_structures::DefaultMemoryImpl;

    fn setup_test_store() -> InvertedIndexStore {
        let memory_manager = MemoryManager::init(DefaultMemoryImpl::default());
        let memory = memory_manager.get(MemoryId::new(0));
        InvertedIndexStore::new(memory)
    }

    #[test]
    fn test_store_and_retrieve_with_standard_match() {
        let mut store = setup_test_store();
        
        // 创建测试数据
        let test_item = InvertedIndexItem {
            keyword: "test".to_string(),
            keyword_group: "group1".to_string(),
            mcp_name: "mcp1".to_string(),
            source_field: "field1".to_string(),
            confidence: 0.95,
            standard_match: "exact".to_string(),
        };

        // 转换为 JSON 并存储
        let json_str = serde_json::to_string(&vec![test_item.clone()]).unwrap();
        store.store_from_json(&json_str).unwrap();

        // 验证存储
        let all_items = store.get_all_items();
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&all_items).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].standard_match, "exact");
    }

    #[test]
    fn test_find_by_keyword_with_standard_match() {
        let mut store = setup_test_store();
        
        // 创建测试数据
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

        // 存储数据
        let json_str = serde_json::to_string(&test_items).unwrap();
        store.store_from_json(&json_str).unwrap();

        // 验证查询
        let result = store.find_by_keyword("test");
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&result).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item.standard_match == "exact"));
        assert!(items.iter().any(|item| item.standard_match == "partial"));
    }

    #[test]
    fn test_find_by_mcp_name_with_standard_match() {
        let mut store = setup_test_store();
        
        // 创建测试数据
        let test_item = InvertedIndexItem {
            keyword: "test".to_string(),
            keyword_group: "group1".to_string(),
            mcp_name: "mcp1".to_string(),
            source_field: "field1".to_string(),
            confidence: 0.95,
            standard_match: "exact".to_string(),
        };

        // 存储数据
        let json_str = serde_json::to_string(&vec![test_item.clone()]).unwrap();
        store.store_from_json(&json_str).unwrap();

        // 验证查询
        let result = store.find_by_mcp_name("mcp1");
        let items: Vec<InvertedIndexItem> = serde_json::from_str(&result).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].standard_match, "exact");
    }

    #[test]
    fn test_validate_json_str() {
        // 测试有效的 JSON
        let valid_json = r#"[{
            "keyword": "test",
            "keyword_group": "group1",
            "mcp_name": "mcp1",
            "source_field": "field1",
            "confidence": 0.95,
            "standard_match": "exact"
        }]"#;
        assert!(validate_json_str(valid_json).is_ok());

        // 测试无效的 JSON（缺少 standard_match）
        let invalid_json = r#"[{
            "keyword": "test",
            "keyword_group": "group1",
            "mcp_name": "mcp1",
            "source_field": "field1",
            "confidence": 0.95
        }]"#;
        assert!(validate_json_str(invalid_json).is_err());

        // 测试无效的 JSON（standard_match 为空）
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