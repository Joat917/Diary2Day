use serde::Serialize;

pub const DATA_CHANGED: &str = "dd://data-changed";
pub const SETTINGS_CHANGED: &str = "dd://settings-changed";
pub const LOCK_STATE: &str = "dd://lock-state";
pub const OP_PROGRESS: &str = "dd://op-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpProgress {
    pub op_id: String,
    pub phase: String,
    pub done: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataChanged {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_key: Option<String>,
}
