use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, REFERER};

/// 봇 차단 방지 설정
#[derive(Debug, Clone)]
pub struct AntiBotConfig {
    pub rotate_user_agent: bool,
    pub add_referer: bool,
    pub add_random_headers: bool,
    pub random_delay_ms: Option<(u64, u64)>,  // (min, max)
}

impl Default for AntiBotConfig {
    fn default() -> Self {
        Self {
            rotate_user_agent: true,
            add_referer: true,
            add_random_headers: true,
            random_delay_ms: Some((500, 2000)),
        }
    }
}

/// 실제 브라우저 User-Agent 목록 (최신 버전)
const USER_AGENTS: &[&str] = &[
    // Chrome on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",

    // Chrome on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",

    // Firefox on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",

    // Firefox on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",

    // Safari on macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",

    // Edge on Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",

    // Chrome on Linux
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",

    // Mobile (iPhone)
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",

    // Mobile (Android)
    "Mozilla/5.0 (Linux; Android 14; SM-S908B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
];

/// 랜덤 User-Agent 가져오기
pub fn get_random_user_agent() -> &'static str {
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

/// 실제 브라우저처럼 보이는 헤더 생성
pub fn build_headers(config: &AntiBotConfig, referer_url: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // User-Agent
    let user_agent = if config.rotate_user_agent {
        get_random_user_agent()
    } else {
        USER_AGENTS[0]
    };
    headers.insert(USER_AGENT, HeaderValue::from_str(user_agent).unwrap());

    // Accept
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
    );

    // Accept-Language
    let languages = [
        "ko-KR,ko;q=0.9,en-US;q=0.8,en;q=0.7",
        "en-US,en;q=0.9,ko;q=0.8",
        "ko-KR,ko;q=0.9",
        "en-US,en;q=0.9",
    ];
    let mut rng = rand::thread_rng();
    let lang_idx = rng.gen_range(0..languages.len());
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static(languages[lang_idx])
    );

    // Accept-Encoding
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate, br")
    );

    // Referer
    if config.add_referer {
        if let Some(referer) = referer_url {
            if let Ok(value) = HeaderValue::from_str(referer) {
                headers.insert(REFERER, value);
            }
        } else {
            // 기본 Referer (구글 검색)
            headers.insert(
                REFERER,
                HeaderValue::from_static("https://www.google.com/")
            );
        }
    }

    // 추가 랜덤 헤더
    if config.add_random_headers {
        // DNT (Do Not Track) - 랜덤하게 추가
        if rng.gen_bool(0.7) {
            headers.insert("dnt", HeaderValue::from_static("1"));
        }

        // Upgrade-Insecure-Requests
        headers.insert(
            "upgrade-insecure-requests",
            HeaderValue::from_static("1")
        );

        // Sec-Fetch-* 헤더 (Chrome/Edge)
        if user_agent.contains("Chrome") || user_agent.contains("Edg") {
            headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
            headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
            headers.insert("sec-fetch-site", HeaderValue::from_static("none"));
            headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
        }

        // sec-ch-ua (Chromium 기반 브라우저)
        if user_agent.contains("Chrome") {
            headers.insert(
                "sec-ch-ua",
                HeaderValue::from_static("\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
            );
            headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
            headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"Windows\""));
        }
    }

    headers
}

/// 랜덤 딜레이 (밀리초)
pub async fn random_delay(min_ms: u64, max_ms: u64) {
    let mut rng = rand::thread_rng();
    let delay = rng.gen_range(min_ms..=max_ms);
    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
}

/// 랜덤 딜레이 (설정 기반)
pub async fn random_delay_from_config(config: &AntiBotConfig) {
    if let Some((min, max)) = config.random_delay_ms {
        random_delay(min, max).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_user_agent() {
        let ua1 = get_random_user_agent();
        let ua2 = get_random_user_agent();

        assert!(!ua1.is_empty());
        assert!(!ua2.is_empty());
        assert!(ua1.starts_with("Mozilla/"));
    }

    #[test]
    fn test_build_headers() {
        let config = AntiBotConfig::default();
        let headers = build_headers(&config, None);

        assert!(headers.contains_key(USER_AGENT));
        assert!(headers.contains_key(ACCEPT));
        assert!(headers.contains_key(ACCEPT_LANGUAGE));
    }

    #[test]
    fn test_build_headers_with_referer() {
        let config = AntiBotConfig::default();
        let headers = build_headers(&config, Some("https://www.aliexpress.com/"));

        assert!(headers.contains_key(REFERER));
        assert_eq!(
            headers.get(REFERER).unwrap().to_str().unwrap(),
            "https://www.aliexpress.com/"
        );
    }
}
