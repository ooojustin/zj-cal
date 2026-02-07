use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[zj-cal] {}", format!($($arg)*))
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum Ctx {
    TimeFetch,
    IcsFetch,
    IcsFetchFile { path: String },
    IcsReadFile { path: String },
}

impl Ctx {
    pub fn into_map(self) -> BTreeMap<String, String> {
        let value = serde_json::to_value(&self).unwrap();
        value
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
            .collect()
    }

    pub fn from_map(map: &BTreeMap<String, String>) -> Result<Self, String> {
        let json_map: serde_json::Map<String, serde_json::Value> = map
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        serde_json::from_value(serde_json::Value::Object(json_map)).map_err(|e| e.to_string())
    }
}
