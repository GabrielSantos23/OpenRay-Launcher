use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    text: String,
    detected_source_language: Option<String>,
}

fn parse(body: &Value) -> Result<Translation, String> {
    let segments = body.get(0).and_then(Value::as_array).ok_or("No translation received")?;
    let text: String = segments.iter().filter_map(|s| s.get(0).and_then(Value::as_str)).collect();
    if text.is_empty() {
        return Err("No translation received".into());
    }
    let detected_source_language = body.get(2).and_then(Value::as_str).map(str::to_owned);
    Ok(Translation { text, detected_source_language })
}

#[tauri::command]
pub async fn translate(text: String, source: String, target: String) -> Result<Translation, String> {
    if text.trim().is_empty() {
        return Err("Text to be translated is empty".into());
    }
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| e.to_string())?;
    let url = url::Url::parse_with_params(ENDPOINT, [("client", "gtx"), ("sl", source.as_str()), ("tl", target.as_str()), ("dt", "t")])
        .map_err(|e| e.to_string())?;
    let response = client
        .post(url)
        .form(&[("q", text.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Translation service returned {}", response.status()));
    }
    let body: Value = serde_json::from_slice(&response.bytes().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    parse(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn joins_segments_and_reads_detected_language() {
        let body = json!([[["Bonjour. ", "Hello. ", null, null, 10], ["Comment ça va ?", "How are you?", null, null, 10]], null, "en", null, null, null, 1.0]);
        assert_eq!(
            parse(&body),
            Ok(Translation { text: "Bonjour. Comment ça va ?".into(), detected_source_language: Some("en".into()) })
        );
    }

    #[test]
    fn skips_segments_without_text() {
        let body = json!([[["Hola", "Hello", null, null, 3], [null, null, "Ola"]], null, "en"]);
        assert_eq!(parse(&body).unwrap().text, "Hola");
    }

    #[test]
    fn rejects_empty_or_malformed_bodies() {
        assert!(parse(&json!([null, null, "en"])).is_err());
        assert!(parse(&json!([[], null, "en"])).is_err());
        assert!(parse(&json!({"error": "bad"})).is_err());
    }

    #[test]
    fn detected_language_is_optional() {
        assert_eq!(parse(&json!([[["Hallo", "Hello"]]])).unwrap().detected_source_language, None);
    }
}
