use serde::Serialize;

pub const DATA_CHANGED: &str = "dd://data-changed";
pub const SETTINGS_CHANGED: &str = "dd://settings-changed";

#[derive(Debug, Clone, Serialize)]
pub struct DataChanged {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_key: Option<String>,
}
