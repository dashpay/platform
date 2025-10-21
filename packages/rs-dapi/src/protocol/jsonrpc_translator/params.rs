use serde_json::Value;

fn parse_bool_flag(value: Option<&Value>, name: &str) -> Result<bool, String> {
    match value {
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::String(s)) if s == "true" => Ok(true),
        Some(Value::String(s)) if s == "false" => Ok(false),
        None | Some(Value::Null) => Ok(false),
        _ => Err(format!("{name} must be boolean")),
    }
}

/// Extract the `height` field from JSON-RPC params, validating numeric bounds.
/// Accepts object-based params and returns friendly error strings for schema issues.
pub fn parse_first_u32_param(params: Option<Value>) -> Result<u32, String> {
    let map = match params {
        Some(Value::Object(map)) => map,
        _ => return Err("params must be object".to_string()),
    };

    let value = map
        .get("height")
        .ok_or_else(|| "must have required property 'height'".to_string())?;
    match value {
        Value::Number(num) => {
            if let Some(raw) = num.as_i64() {
                if raw < 0 {
                    return Err("params/height must be >= 0".to_string());
                }
                if raw > i64::from(u32::MAX) {
                    return Err("params/height must be <= 4294967295".to_string());
                }
                Ok(raw as u32)
            } else if let Some(raw) = num.as_u64() {
                if raw > u32::MAX as u64 {
                    return Err("params/height must be <= 4294967295".to_string());
                }
                Ok(raw as u32)
            } else {
                Err("params/height must be integer".to_string())
            }
        }
        _ => Err("params/height must be integer".to_string()),
    }
}

/// Parse raw transaction parameters, supporting string or array forms with fee flags.
/// Returns the decoded bytes plus `allow_high_fees` and `bypass_limits` toggles.
pub fn parse_send_raw_tx_params(params: Option<Value>) -> Result<(Vec<u8>, bool, bool), String> {
    match params {
        Some(Value::Array(a)) => {
            if a.is_empty() {
                return Err("missing raw transaction parameter".to_string());
            }
            let raw_hex = a[0]
                .as_str()
                .ok_or_else(|| "raw transaction must be a hex string".to_string())?;
            let tx = hex::decode(raw_hex)
                .map_err(|_| "raw transaction must be valid hex".to_string())?;

            let allow_high_fees = parse_bool_flag(a.get(1), "allow_high_fees")?;
            let bypass_limits = parse_bool_flag(a.get(2), "bypass_limits")?;

            Ok((tx, allow_high_fees, bypass_limits))
        }
        Some(Value::String(s)) => {
            let tx =
                hex::decode(&s).map_err(|_| "raw transaction must be valid hex".to_string())?;
            Ok((tx, false, false))
        }
        _ => Err("params must be an array or hex string".to_string()),
    }
}
