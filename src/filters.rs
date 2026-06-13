use std::collections::HashMap;

use serde_json::Value;
use tera::from_value;

pub fn register(tera: &mut tera::Tera) {
    tera.register_filter("nohash", nohash);
    tera.register_filter("to_rgb", to_rgb);
    tera.register_filter("to_chrome", to_chrome);
    tera.register_filter("to_apple", to_apple);
    tera.register_filter("upper", upper);
    tera.register_filter("lower", lower);
}

fn nohash(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    Ok(Value::String(s.trim_start_matches('#').to_string()))
}

fn to_rgb(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    Ok(Value::String(format!("({},{},{})", r, g, b)))
}

fn to_chrome(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    Ok(Value::String(format!("[{}, {}, {}]", r, g, b)))
}

fn to_apple(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|e| tera::Error::msg(e.to_string()))?;
    let a = args
        .get("alpha")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .min(255) as u8;
    let to_apple = |x: u8| (x as u32) * 257;
    Ok(Value::String(format!(
        "{{{}, {}, {}, {}}}",
        to_apple(r),
        to_apple(g),
        to_apple(b),
        to_apple(a)
    )))
}

fn upper(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    Ok(Value::String(s.to_uppercase()))
}

fn lower(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = from_value::<String>(value.clone())?;
    Ok(Value::String(s.to_lowercase()))
}
