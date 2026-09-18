use serde_json::{json, Value};

pub const TOOL_NAMES: [&str; 4] = ["gate_lock", "gate_push_prompt", "gate_status", "gate_due"];

pub const SESSION_RESOURCE_URI: &str = "recallgate://session";

pub fn tool_names() -> Vec<&'static str> {
    TOOL_NAMES.to_vec()
}

pub fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "gate_lock",
                "description": "Start a session freeze when cadence and lock backend allow.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "prompt_id": { "type": "string" },
                        "reason": { "type": "string", "enum": ["idle", "manual", "rpc"] }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "gate_push_prompt",
                "description": "Add a multiple-choice card to the deck without locking.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "stem": { "type": "string" },
                        "choices": {
                            "type": "array",
                            "items": { "type": "string" },
                            "minItems": 4,
                            "maxItems": 4
                        },
                        "correct_index": { "type": "integer", "minimum": 0, "maximum": 3 },
                        "external_ref": { "type": "string" }
                    },
                    "required": ["stem", "choices", "correct_index"],
                    "additionalProperties": false
                }
            },
            {
                "name": "gate_status",
                "description": "Read gate phase, capability, and cooldown.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "gate_due",
                "description": "List prompt ids that may appear on the lock surface.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        ]
    })
}

pub fn resources_list() -> Value {
    json!({
        "resources": [{
            "uri": SESSION_RESOURCE_URI,
            "name": "session",
            "description": "Current gate phase and capability (same as gate_status).",
            "mimeType": "application/json"
        }]
    })
}
