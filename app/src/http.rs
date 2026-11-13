use serde_json::Value;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

const LIMIT: u64 = 4 * 1024 * 1024;

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .http_status_as_error(false)
        .build()
        .into()
}

pub enum Failure {
    Offline,
    Status(u16),
    Body(String),
}

fn read(response: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<(u16, Vec<u8>), Failure> {
    let mut response = response.map_err(|_| Failure::Offline)?;
    let status = response.status().as_u16();
    let bytes = response.body_mut().with_config().limit(LIMIT).read_to_vec().map_err(|e| Failure::Body(e.to_string()))?;
    Ok((status, bytes))
}

pub fn get(url: &str) -> Result<(u16, Vec<u8>), Failure> {
    read(agent().get(url).call())
}

pub fn get_json(url: &str) -> Result<Option<Value>, Failure> {
    match get(url)? {
        (404, _) => Ok(None),
        (200..=299, bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| Failure::Body(e.to_string())),
        (status, _) => Err(Failure::Status(status)),
    }
}

pub fn download(url: &str, limit: u64, timeout: std::time::Duration) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .timeout_global(Some(timeout))
        .build()
        .into();
    let mut response = agent.get(url).header("User-Agent", "Openray-Updater").call().map_err(|e| format!("Download failed: {e}"))?;
    response.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| format!("Download failed: {e}"))
}

pub fn post_form(url: &str, fields: &[(&str, &str)]) -> Result<Value, Failure> {
    match read(agent().post(url).send_form(fields.iter().copied()))? {
        (200..=299, bytes) => serde_json::from_slice(&bytes).map_err(|e| Failure::Body(e.to_string())),
        (status, _) => Err(Failure::Status(status)),
    }
}
