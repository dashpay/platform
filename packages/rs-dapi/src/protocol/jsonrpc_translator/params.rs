use serde_json::Value;

pub fn parse_first_u32_param(params: Option<Value>) -> Result<u32, String> {
    match params {
        Some(Value::Array(a)) => {
            if a.is_empty() {
                return Err("missing required parameter".to_string());
            }
            parse_u32_from_value(&a[0])
        }
        Some(Value::Object(map)) => {
            let mut last_error = Some("object must contain a numeric value".to_string());
            for value in map.values() {
                match parse_u32_from_value(value) {
                    Ok(v) => return Ok(v),
                    Err(e) => last_error = Some(e),
                }
            }
            Err(last_error.expect("object must contain a numeric value"))
        }
        _ => Err("params must be an array or object".to_string()),
    }
}

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

            let allow_high_fees = a.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
            let bypass_limits = a.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
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

fn parse_u32_from_value(value: &Value) -> Result<u32, String> {
    match value {
        Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| "value must be a non-negative integer".to_string())
            .and_then(|v| {
                if v <= u32::MAX as u64 {
                    Ok(v as u32)
                } else {
                    Err("value out of range".to_string())
                }
            }),
        _ => Err("value must be a number".to_string()),
    }
}
