use serde_json::Value;
use std::time::Duration;

const ENDPOINT: &str = "https://api.dictionaryapi.dev/api/v2/entries/en/";
const ATTEMPTS: u32 = 3;

#[tauri::command]
pub async fn dictionary_define(word: String) -> Result<Value, String> {
    let word = word.trim();
    if word.is_empty() {
        return Err("Type a word to define".into());
    }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| e.to_string())?;
    let url = format!("{ENDPOINT}{}", percent_encoding::utf8_percent_encode(word, percent_encoding::NON_ALPHANUMERIC));
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600)).await;
        }
        match client.get(&url).send().await {
            Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND => return Ok(Value::Array(Vec::new())),
            Ok(r) if r.status().is_success() => return serde_json::from_slice(&r.bytes().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string()),
            Ok(r) => last = format!("The dictionary service returned an error ({}).", r.status().as_u16()),
            Err(_) => last = "The dictionary service didn't respond. Check your connection and try again.".into(),
        }
    }
    Err(last)
}
