//! HTTP 요청 실행 계층.
//!
//! UI 스레드를 막지 않도록 요청은 별도 스레드에서 실행하고,
//! 결과는 `mpsc` 채널을 통해 앱으로 전달한다.

use std::io::Read;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// 응답 본문을 메모리에 담는 상한. 초과분은 잘라 OOM/UI 멈춤을 방지한다.
const MAX_BODY_BYTES: u64 = 50 * 1024 * 1024; // 50 MiB

/// 전체 요청 타임아웃.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// 연결(핸드셰이크) 타임아웃.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 지원하는 HTTP 메서드.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub const ALL: [Method; 7] = [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
        Method::Head,
        Method::Options,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    fn to_reqwest(self) -> reqwest::Method {
        match self {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
            Method::Head => reqwest::Method::HEAD,
            Method::Options => reqwest::Method::OPTIONS,
        }
    }

    /// 바디를 보낼 수 있는 메서드인지 여부.
    pub fn allows_body(&self) -> bool {
        matches!(self, Method::Post | Method::Put | Method::Patch | Method::Delete)
    }
}

/// 키-값 쌍 (헤더 / 쿼리 파라미터 공용).
#[derive(Debug, Clone, Default)]
pub struct KeyValue {
    pub enabled: bool,
    pub key: String,
    pub value: String,
}

impl KeyValue {
    pub fn new() -> Self {
        Self { enabled: true, key: String::new(), value: String::new() }
    }
}

/// UI에서 조립한 요청 명세.
#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub method: Method,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub query: Vec<KeyValue>,
    pub body: String,
}

/// 요청 결과.
#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub elapsed_ms: u128,
    pub size_bytes: usize,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// 본문이 JSON으로 파싱되어 pretty-print 가능한 경우 그 결과.
    pub pretty_body: Option<String>,
    /// 본문이 크기 상한(`MAX_BODY_BYTES`)을 초과해 잘렸는지 여부.
    pub truncated: bool,
}

/// 채널로 전달되는 실행 결과.
pub enum FetchResult {
    Ok(ResponseData),
    Err(String),
}

/// 별도 스레드에서 요청을 실행하고 결과를 채널로 보낸다.
///
/// `ctx`는 응답 도착 시 UI를 다시 그리도록 요청하는 데 쓰인다.
pub fn execute(spec: RequestSpec, tx: Sender<FetchResult>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let result = run_blocking(&spec);
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

fn run_blocking(spec: &RequestSpec) -> FetchResult {
    let client = match reqwest::blocking::Client::builder()
        .user_agent("postwoman/0.1")
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
    {
        Ok(c) => c,
        Err(e) => return FetchResult::Err(format!("클라이언트 생성 실패: {e}")),
    };

    // URL 파싱 및 쿼리 파라미터 병합.
    let mut url = match reqwest::Url::from_str(spec.url.trim()) {
        Ok(u) => u,
        Err(e) => return FetchResult::Err(format!("잘못된 URL: {e}")),
    };
    {
        let mut pairs = url.query_pairs_mut();
        for kv in &spec.query {
            if kv.enabled && !kv.key.is_empty() {
                pairs.append_pair(&kv.key, &kv.value);
            }
        }
    }

    let mut req = client.request(spec.method.to_reqwest(), url);

    for kv in &spec.headers {
        if kv.enabled && !kv.key.is_empty() {
            req = req.header(kv.key.trim(), kv.value.trim());
        }
    }

    if spec.method.allows_body() && !spec.body.trim().is_empty() {
        req = req.body(spec.body.clone());
    }

    let start = Instant::now();
    let resp = match req.send() {
        Ok(r) => r,
        Err(e) => return FetchResult::Err(format!("요청 실패: {e}")),
    };

    let status = resp.status();
    let status_text = status
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let mut headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect();
    headers.sort_by(|a, b| a.0.cmp(&b.0));

    // 상한 + 1바이트까지만 읽어, 초과 여부를 판별하면서도 메모리 사용을 제한한다.
    // (Content-Length는 위조될 수 있으므로 실제 읽은 바이트로 판단한다.)
    let mut buf = Vec::new();
    if let Err(e) = resp.take(MAX_BODY_BYTES + 1).read_to_end(&mut buf) {
        return FetchResult::Err(format!("본문 읽기 실패: {e}"));
    }
    let truncated = buf.len() as u64 > MAX_BODY_BYTES;
    if truncated {
        buf.truncate(MAX_BODY_BYTES as usize);
    }
    // text() 대신 직접 읽었으므로 UTF-8 디코딩은 손실 허용으로 처리한다.
    let body = String::from_utf8_lossy(&buf).into_owned();
    let elapsed_ms = start.elapsed().as_millis();
    let size_bytes = buf.len();

    // 잘린 본문은 불완전하므로 JSON pretty-print를 시도하지 않는다.
    let pretty_body = if truncated {
        None
    } else {
        serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
    };

    FetchResult::Ok(ResponseData {
        status: status.as_u16(),
        status_text,
        elapsed_ms,
        size_bytes,
        headers,
        body,
        pretty_body,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(key: &str, value: &str) -> KeyValue {
        KeyValue { enabled: true, key: key.into(), value: value.into() }
    }

    /// 실제 GET 요청 + 쿼리 파라미터 병합 + JSON pretty-print 경로를 검증한다.
    /// (네트워크가 필요하므로 `--ignored`로 명시 실행)
    #[test]
    #[ignore = "requires network"]
    fn get_with_query_merges_and_parses_json() {
        let spec = RequestSpec {
            method: Method::Get,
            url: "https://httpbin.org/get".into(),
            headers: vec![kv("X-Test", "postwoman")],
            query: vec![kv("foo", "bar")],
            body: String::new(),
        };
        match run_blocking(&spec) {
            FetchResult::Ok(r) => {
                assert_eq!(r.status, 200, "기대 200, 실제 {}", r.status);
                assert!(r.pretty_body.is_some(), "JSON 본문이 pretty-print 되어야 함");
                // httpbin은 받은 쿼리/헤더를 본문에 echo 한다.
                assert!(r.body.contains("\"foo\""), "쿼리 파라미터가 전송돼야 함");
                assert!(r.body.contains("bar"));
                assert!(r.body.contains("postwoman"), "커스텀 헤더가 전송돼야 함");
            }
            FetchResult::Err(e) => panic!("요청 실패: {e}"),
        }
    }

    /// POST 바디 전송 경로 검증.
    #[test]
    #[ignore = "requires network"]
    fn post_sends_body() {
        let spec = RequestSpec {
            method: Method::Post,
            url: "https://httpbin.org/post".into(),
            headers: vec![kv("Content-Type", "application/json")],
            query: vec![],
            body: r#"{"hello":"world"}"#.into(),
        };
        match run_blocking(&spec) {
            FetchResult::Ok(r) => {
                assert_eq!(r.status, 200);
                assert!(r.body.contains("hello") && r.body.contains("world"));
            }
            FetchResult::Err(e) => panic!("요청 실패: {e}"),
        }
    }

    #[test]
    fn invalid_url_is_reported() {
        let spec = RequestSpec {
            method: Method::Get,
            url: "not a url".into(),
            headers: vec![],
            query: vec![],
            body: String::new(),
        };
        assert!(matches!(run_blocking(&spec), FetchResult::Err(_)));
    }
}
