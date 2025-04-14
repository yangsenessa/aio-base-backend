use candid::{CandidType, Decode, Encode};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableVec};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use serde_json::{Value, json};

type Memory = VirtualMemory<DefaultMemoryImpl>;

/// A wrapper around Vec<String> that implements Storable
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StringVec(pub Vec<String>);

impl Storable for StringVec {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(&self.0).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Self(Decode!(bytes.as_ref(), Vec<String>).unwrap())
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 64, is_fixed_size: false };
}

/// Method parameter schema definition
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct InputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, SchemaProperty>,
}

/// Property in schema
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Method definition
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Method {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_params: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<InputSchema>,
}

/// Source information
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Source {
    pub author: String,
    pub version: String,
    pub github: String,
}

/// AioIndex represents an index item in the system
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct AioIndex {
    pub id: String,           // agent/mcp name
    pub description: String,
    pub author: String,
    pub version: String,
    pub github: String,
    pub transport: Vec<String>,
    pub methods: Vec<Method>,
    pub source: Source,
    pub keywords: Vec<String>,
    pub scenarios: Vec<String>,
}

impl Default for AioIndex {
    fn default() -> Self {
        Self {
            id: String::new(),
            description: String::new(),
            author: String::new(),
            version: String::new(),
            github: String::new(),
            transport: Vec::new(),
            methods: Vec::new(),
            source: Source {
                author: String::new(),
                version: String::new(),
                github: String::new(),
            },
            keywords: Vec::new(),
            scenarios: Vec::new(),
        }
    }
}

// Implement Storable for AioIndex
impl ic_stable_structures::Storable for AioIndex {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded { max_size: 1024 * 128, is_fixed_size: false };
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    
    static AIO_INDICES: RefCell<StableBTreeMap<String, AioIndex, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
        )
    );
    
    static KEYWORD_INDEX: RefCell<StableBTreeMap<String, StringVec, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6)))
        )
    );
}

/// Manager for AioIndex storage and operations
pub struct AioIndexManager;

impl AioIndexManager {
    /// Create a new instance of the manager
    pub fn new() -> Self {
        Self {}
    }

    /// Create a new AioIndex
    pub fn create(&self, index: AioIndex) -> Result<(), String> {
        let id = index.id.clone();
        
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if indices.contains_key(&id) {
                return Err(format!("Index with ID {} already exists", id));
            }
            // Log the index being created
            ic_cdk::println!("Creating new AioIndex: id={}, description={}, keywords={:?}", 
                id, index.description, index.keywords);
            indices.insert(id.clone(), index.clone());
            
            // Add to keyword indices
            for keyword in &index.keywords {
                self.add_to_keyword_index(keyword, &id);
            }
            
            Ok(())
        })
    }

    /// Helper function to add an index to the keyword index
    fn add_to_keyword_index(&self, keyword: &str, id: &str) {
        KEYWORD_INDEX.with(|keyword_index| {
            let mut keyword_index = keyword_index.borrow_mut();
            let keyword_lower = keyword.to_lowercase();
            
            let mut index_list = keyword_index
                .get(&keyword_lower)
                .map(|v| v.0.clone())
                .unwrap_or_default();
            
            if !index_list.contains(&id.to_string()) {
                index_list.push(id.to_string());
                keyword_index.insert(keyword_lower, StringVec(index_list));
            }
        });
    }
    pub fn read(&self, id: &str) -> Option<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.get(&id.to_string())
        })
    }

    /// Update an existing AioIndex
    pub fn update(&self, id: &str, updated_index: AioIndex) -> Result<(), String> {
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if !indices.contains_key(&id.to_string()) {
                return Err(format!("Index with ID {} does not exist", id));
            }
            
            // Get the old index to update keyword references
            if let Some(old_index) = indices.get(&id.to_string()) {
                // Remove from old keywords
                for keyword in &old_index.keywords {
                    self.remove_from_keyword_index(keyword, id);
                }
            }
            
            // Add to new keywords
            for keyword in &updated_index.keywords {
                self.add_to_keyword_index(keyword, id);
            }
            
            // Update the index
            indices.insert(id.to_string(), updated_index);
            Ok(())
        })
    }

    /// Delete an AioIndex by ID
    pub fn delete(&self, id: &str) -> Result<(), String> {
        AIO_INDICES.with(|indices| {
            let mut indices = indices.borrow_mut();
            
            if !indices.contains_key(&id.to_string()) {
                return Err(format!("Index with ID {} does not exist", id));
            }
            
            // Get the index to remove its keywords
            if let Some(index) = indices.get(&id.to_string()) {
                // Remove from keyword indices
                for keyword in &index.keywords {
                    self.remove_from_keyword_index(keyword, id);
                }
            }
            
            // Remove the index
            indices.remove(&id.to_string());
            Ok(())
        })
    }

    /// Helper function to remove an index from the keyword index
    fn remove_from_keyword_index(&self, keyword: &str, id: &str) {
        KEYWORD_INDEX.with(|keyword_index| {
            let mut keyword_index = keyword_index.borrow_mut();
            let keyword_lower = keyword.to_lowercase();
            
            if let Some(index_list) = keyword_index.get(&keyword_lower) {
                let mut new_list = index_list.0.clone();
                new_list.retain(|item_id| item_id != id);
                
                if new_list.is_empty() {
                    keyword_index.remove(&keyword_lower);
                } else {
                    keyword_index.insert(keyword_lower, StringVec(new_list));
                }
            }
        });
    }

    /// List all indices
    pub fn list_all(&self) -> Vec<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.iter().map(|(_, value)| value).collect()
        })
    }

    /// Get indices with pagination
    pub fn get_indices_paginated(&self, offset: usize, limit: usize) -> Vec<AioIndex> {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.iter()
                .skip(offset)
                .take(limit)
                .map(|(_, value)| value)
                .collect()
        })
    }

    /// Parse JSON and create an AioIndex
    pub fn create_from_json(&self,name:&str, json_str: &str) -> Result<(), String> {
        let parsed: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON parsing error: {}", e))?;
        
        let obj = parsed.as_object()
            .ok_or_else(|| "Invalid JSON: expected object".to_string())?;
        
        let mcp_id = name.to_string();
        
        let description = obj.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let author = obj.get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let version = obj.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let github = obj.get("github")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let transport = obj.get("transport")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse methods
        let methods = obj.get("methods")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let method_obj = item.as_object()?;
                    
                    let name = method_obj.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("").to_string();
                    
                    let description = method_obj.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("").to_string();
                    
                    let required_params = method_obj.get("required_params")
                        .and_then(|v| v.as_array())
                        .map(|params| {
                            params.iter()
                                .filter_map(|param| param.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        });
                    
                    let input_schema = method_obj.get("inputSchema")
                        .and_then(|v| v.as_object())
                        .map(|schema| {
                            let schema_type = schema.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("object").to_string();
                            
                            let properties = schema.get("properties")
                                .and_then(|v| v.as_object())
                                .map(|props| {
                                    props.iter().filter_map(|(key, value)| {
                                        let prop_obj = value.as_object()?;
                                        
                                        let prop_type = prop_obj.get("type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("string").to_string();
                                        
                                        let description = prop_obj.get("description")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        
                                        let default = prop_obj.get("default").map(|v| v.to_string());
                                        
                                        let enum_values = prop_obj.get("enum")
                                            .and_then(|v| v.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                                    .collect::<Vec<String>>()
                                            });
                                        
                                        Some((key.clone(), SchemaProperty {
                                            property_type: prop_type,
                                            description,
                                            default,
                                            enum_values,
                                        }))
                                    }).collect::<HashMap<String, SchemaProperty>>()
                                })
                                .unwrap_or_else(HashMap::new);
                            
                            InputSchema {
                                schema_type,
                                properties,
                            }
                        });
                    
                    Some(Method {
                        name,
                        description,
                        required_params,
                        input_schema,
                    })
                }).collect::<Vec<Method>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse source
        let source = obj.get("source")
            .and_then(|v| v.as_object())
            .map(|src| {
                Source {
                    author: src.get("author").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    version: src.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    github: src.get("github").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                }
            })
            .unwrap_or_else(|| Source {
                author: String::new(),
                version: String::new(),
                github: String::new(),
            });
        
        // Parse keywords
        let keywords = obj.get("keyword")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Parse scenarios
        let scenarios = obj.get("scenario")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);
        
        // Create and store the index
        let aio_index = AioIndex {
            id: mcp_id.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            version: version.to_string(),
            github: github.to_string(),
            transport,
            methods,
            source,
            keywords,
            scenarios,
        };
        
        self.create(aio_index)
    }
    
    /// Search for indices by keyword
    pub fn search_by_keyword(&self, keyword: &str) -> Vec<AioIndex> {
        let keyword_lower = keyword.to_lowercase();
        let mut result = Vec::new();
        
        KEYWORD_INDEX.with(|keyword_index| {
            let keyword_index = keyword_index.borrow();
            
            // Check for exact keyword match
            if let Some(ids) = keyword_index.get(&keyword_lower) {
                for id in &ids.0 {
                    if let Some(index) = self.read(id) {
                        result.push(index);
                    }
                }
            }
            
            // Check for partial keyword matches
            for (key, index_ids) in keyword_index.iter() {
                if key.contains(&keyword_lower) && *key != keyword_lower {
                    for id in &index_ids.0 {
                        let index = self.read(id);
                        if let Some(index_item) = index {
                            if !result.iter().any(|i| i.id == index_item.id) {
                                result.push(index_item);
                            }
                        }
                    }
                }
            }
        });
        
        result
    }

    /// Get index as JSON string
    pub fn get_json(&self, id: &str) -> Result<String, String> {
        let index = self.read(id).ok_or_else(|| format!("Index with ID {} not found", id))?;
        serde_json::to_string(&index).map_err(|e| format!("Failed to convert index to JSON: {}", e))
    }
    
    pub fn search_full_text(&self, query: &str) -> Vec<AioIndex> {
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            
            for (_, index) in indices.iter() {
                // Check ID
                let id_match = index.id.to_lowercase().contains(&query_lower);
                
                // Check description
                let desc_match = index.description.to_lowercase().contains(&query_lower);
                
                // Check author
                let author_match = index.author.to_lowercase().contains(&query_lower);
                
                // Check methods
                let methods_match = index.methods.iter().any(|method| {
                    method.name.to_lowercase().contains(&query_lower) || 
                    method.description.to_lowercase().contains(&query_lower)
                });
                
                // Check keywords
                let keywords_match = index.keywords.iter().any(|keyword| {
                    keyword.to_lowercase().contains(&query_lower)
                });
                
                // Check scenarios
                let scenarios_match = index.scenarios.iter().any(|scenario| {
                    scenario.to_lowercase().contains(&query_lower)
                });
                
                if id_match || desc_match || author_match || methods_match || keywords_match || scenarios_match {
                    seen_ids.insert(index.id.clone());
                    results.push(index.clone());
                }
            }
        });
        
        results
    }
    
    /// Get the count of all indices
    pub fn count(&self) -> usize {
        AIO_INDICES.with(|indices| {
            let indices = indices.borrow();
            indices.len() as usize
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aio_index() {
        let manager = AioIndexManager::new();
        // Assuming there's a previous test that creates this index
        let result = manager.read("1743948342885");
        assert!(result.is_some());
        let index = result.unwrap();
        assert_eq!(index.id, "1743948342885");
        assert_eq!(index.description, "此服务是提供memory相关的mcp服务");
        assert_eq!(index.keywords.len(), 2);
    }
}
