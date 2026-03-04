use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmNode {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub attrs: Option<Value>,
    #[serde(default)]
    pub content: Option<Vec<PmNode>>,
    #[serde(default)]
    pub marks: Option<Vec<PmMark>>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmMark {
    #[serde(rename = "type")]
    pub mark_type: String,
    #[serde(default)]
    pub attrs: Option<Value>,
}
