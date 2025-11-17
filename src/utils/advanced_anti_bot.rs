use rand::Rng;
use rand_distr::{Distribution, Normal};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE, ACCEPT_ENCODING, REFERER};
use std::collections::HashMap;

/// 브라우저 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
}

/// 브라우저 프로필 (일관된 헤더 세트)
#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub browser_type: BrowserType,
    pub user_agent: String,
    pub platform: String,
    pub accept_language: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl BrowserProfile {
    /// 랜덤 Chrome 프로필 생성
    pub fn random_chrome() -> Self {
        let mut rng = rand::thread_rng();

        let versions = ["120.0.0.0", "119.0.0.0", "118.0.0.0"];
        let version = versions[rng.gen_range(0..versions.len())];

        let platforms = [
            ("Windows NT 10.0; Win64; x64", "Win32"),
            ("Windows NT 11.0; Win64; x64", "Win32"),
            ("Macintosh; Intel Mac OS X 10_15_7", "MacIntel"),
        ];
        let (os, platform) = platforms[rng.gen_range(0..platforms.len())];

        let languages = [
            "ko-KR,ko;q=0.9,en-US;q=0.8,en;q=0.7",
            "en-US,en;q=0.9,ko;q=0.8",
            "ko-KR,ko;q=0.9",
        ];

        let viewports = [
            (1920, 1080),
            (1366, 768),
            (1536, 864),
            (1440, 900),
        ];
        let (width, height) = viewports[rng.gen_range(0..viewports.len())];

        Self {
            browser_type: BrowserType::Chrome,
            user_agent: format!(
                "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{} Safari/537.36",
                os, version
            ),
            platform: platform.to_string(),
            accept_language: languages[rng.gen_range(0..languages.len())].to_string(),
            viewport_width: width,
            viewport_height: height,
        }
    }

    /// 랜덤 Firefox 프로필 생성
    pub fn random_firefox() -> Self {
        let mut rng = rand::thread_rng();

        let versions = ["121.0", "120.0", "119.0"];
        let version = versions[rng.gen_range(0..versions.len())];

        let platforms = [
            ("Windows NT 10.0; Win64; x64; rv", "Win32"),
            ("Macintosh; Intel Mac OS X 10.15; rv", "MacIntel"),
            ("X11; Linux x86_64; rv", "Linux x86_64"),
        ];
        let (os, platform) = platforms[rng.gen_range(0..platforms.len())];

        let languages = [
            "ko-KR,ko;q=0.9,en-US;q=0.8,en;q=0.7",
            "en-US,en;q=0.9",
        ];

        let viewports = [
            (1920, 1080),
            (1366, 768),
            (1536, 864),
        ];
        let (width, height) = viewports[rng.gen_range(0..viewports.len())];

        Self {
            browser_type: BrowserType::Firefox,
            user_agent: format!("Mozilla/5.0 ({}:{}) Gecko/20100101 Firefox/{}", os, version, version),
            platform: platform.to_string(),
            accept_language: languages[rng.gen_range(0..languages.len())].to_string(),
            viewport_width: width,
            viewport_height: height,
        }
    }

    /// 랜덤 Safari 프로필 생성
    pub fn random_safari() -> Self {
        let mut rng = rand::thread_rng();

        let versions = ["17.1", "17.0", "16.6"];
        let version = versions[rng.gen_range(0..versions.len())];

        let viewports = [
            (1920, 1080),
            (1440, 900),
        ];
        let (width, height) = viewports[rng.gen_range(0..viewports.len())];

        Self {
            browser_type: BrowserType::Safari,
            user_agent: format!(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{} Safari/605.1.15",
                version
            ),
            platform: "MacIntel".to_string(),
            accept_language: "en-US,en;q=0.9".to_string(),
            viewport_width: width,
            viewport_height: height,
        }
    }

    /// 랜덤 프로필 생성
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let choice = rng.gen_range(0..100);

        // Chrome: 70%, Firefox: 20%, Safari: 10%
        if choice < 70 {
            Self::random_chrome()
        } else if choice < 90 {
            Self::random_firefox()
        } else {
            Self::random_safari()
        }
    }

    /// 프로필에 맞는 헤더 생성
    pub fn build_headers(&self, referer: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // User-Agent
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent).unwrap());

        // Accept (브라우저별로 다름)
        let accept = match self.browser_type {
            BrowserType::Chrome | BrowserType::Edge => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
            }
            BrowserType::Firefox => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
            }
            BrowserType::Safari => {
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
            }
        };
        headers.insert(ACCEPT, HeaderValue::from_static(accept));

        // Accept-Language
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_str(&self.accept_language).unwrap()
        );

        // Accept-Encoding
        headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br")
        );

        // Referer
        if let Some(ref_url) = referer {
            if let Ok(value) = HeaderValue::from_str(ref_url) {
                headers.insert(REFERER, value);
            }
        }

        // 브라우저별 추가 헤더
        match self.browser_type {
            BrowserType::Chrome | BrowserType::Edge => {
                // Chromium 기반 브라우저
                headers.insert("dnt", HeaderValue::from_static("1"));
                headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
                headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
                headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
                headers.insert("sec-fetch-site", HeaderValue::from_static("none"));
                headers.insert("sec-fetch-user", HeaderValue::from_static("?1"));
                headers.insert(
                    "sec-ch-ua",
                    HeaderValue::from_static("\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
                );
                headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));

                if self.platform == "Win32" {
                    headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"Windows\""));
                } else if self.platform == "MacIntel" {
                    headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
                } else {
                    headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"Linux\""));
                }
            }
            BrowserType::Firefox => {
                // Firefox는 Sec-Fetch-* 헤더 사용 안함
                headers.insert("dnt", HeaderValue::from_static("1"));
                headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
            }
            BrowserType::Safari => {
                // Safari는 최소한의 헤더 사용
                headers.insert("upgrade-insecure-requests", HeaderValue::from_static("1"));
            }
        }

        headers
    }
}

/// 고급 봇 차단 방지 설정
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdvancedAntiBotConfig {
    pub use_browser_profiles: bool,          // 브라우저 프로필 사용
    pub rotate_profiles: bool,               // 프로필 로테이션
    pub session_consistency: bool,           // 세션 일관성 유지
    pub intelligent_timing: bool,            // 지능형 타이밍 (가우시안 분포)
    pub simulate_assets: bool,               // CSS/JS/이미지 요청 시뮬레이션
    pub natural_navigation: bool,            // 자연스러운 네비게이션 경로
    pub min_delay_secs: f64,                 // 최소 딜레이 (초)
    pub max_delay_secs: f64,                 // 최대 딜레이 (초)
    pub std_dev: f64,                        // 표준편차
}

impl Default for AdvancedAntiBotConfig {
    fn default() -> Self {
        Self {
            use_browser_profiles: true,
            rotate_profiles: false,  // 세션 내에서는 동일 프로필 유지
            session_consistency: true,
            intelligent_timing: true,
            simulate_assets: false,  // 기본적으로는 비활성화 (느림)
            natural_navigation: true,
            min_delay_secs: 1.0,
            max_delay_secs: 3.0,
            std_dev: 0.5,
        }
    }
}

impl AdvancedAntiBotConfig {
    /// 빠른 크롤링 (덜 안전)
    pub fn fast() -> Self {
        Self {
            min_delay_secs: 0.5,
            max_delay_secs: 1.5,
            std_dev: 0.3,
            simulate_assets: false,
            ..Default::default()
        }
    }

    /// 느린 크롤링 (더 안전)
    pub fn slow() -> Self {
        Self {
            min_delay_secs: 3.0,
            max_delay_secs: 8.0,
            std_dev: 1.0,
            simulate_assets: true,
            ..Default::default()
        }
    }

    /// 가우시안 분포 기반 지능형 딜레이
    pub async fn intelligent_delay(&self) {
        if !self.intelligent_timing {
            // 단순 랜덤
            let mut rng = rand::thread_rng();
            let delay = rng.gen_range(self.min_delay_secs..=self.max_delay_secs);
            tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay)).await;
            return;
        }

        // 가우시안 분포 (정규분포)
        let mean = (self.min_delay_secs + self.max_delay_secs) / 2.0;
        let std_dev = self.std_dev;

        let normal = Normal::new(mean, std_dev).unwrap();
        let mut rng = rand::thread_rng();

        // 범위를 벗어나지 않도록 클램핑
        let delay = normal.sample(&mut rng)
            .max(self.min_delay_secs)
            .min(self.max_delay_secs);

        tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay)).await;
    }
}

/// 세션 관리 (일관성 유지)
#[derive(Debug, Clone)]
pub struct CrawlSessionState {
    pub profile: BrowserProfile,
    pub visited_urls: Vec<String>,
    pub current_referer: Option<String>,
    pub session_start: std::time::Instant,
}

impl CrawlSessionState {
    pub fn new() -> Self {
        Self {
            profile: BrowserProfile::random(),
            visited_urls: Vec::new(),
            current_referer: None,
            session_start: std::time::Instant::now(),
        }
    }

    pub fn new_with_profile(profile: BrowserProfile) -> Self {
        Self {
            profile,
            visited_urls: Vec::new(),
            current_referer: None,
            session_start: std::time::Instant::now(),
        }
    }

    /// URL 방문 기록
    pub fn visit_url(&mut self, url: String) {
        self.current_referer = self.visited_urls.last().cloned();
        self.visited_urls.push(url);
    }

    /// 세션 지속 시간
    pub fn session_duration(&self) -> std::time::Duration {
        self.session_start.elapsed()
    }

    /// 일관된 헤더 생성
    pub fn build_headers(&self) -> HeaderMap {
        self.profile.build_headers(self.current_referer.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_profiles() {
        let chrome = BrowserProfile::random_chrome();
        assert_eq!(chrome.browser_type, BrowserType::Chrome);
        assert!(chrome.user_agent.contains("Chrome"));

        let firefox = BrowserProfile::random_firefox();
        assert_eq!(firefox.browser_type, BrowserType::Firefox);
        assert!(firefox.user_agent.contains("Firefox"));

        let safari = BrowserProfile::random_safari();
        assert_eq!(safari.browser_type, BrowserType::Safari);
        assert!(safari.user_agent.contains("Safari"));
    }

    #[test]
    fn test_header_consistency() {
        let profile = BrowserProfile::random_chrome();
        let headers1 = profile.build_headers(None);
        let headers2 = profile.build_headers(None);

        // 같은 프로필은 같은 헤더 생성
        assert_eq!(
            headers1.get(USER_AGENT).unwrap(),
            headers2.get(USER_AGENT).unwrap()
        );
    }

    #[test]
    fn test_session_state() {
        let mut session = CrawlSessionState::new();

        session.visit_url("https://example.com/page1".to_string());
        assert_eq!(session.visited_urls.len(), 1);
        assert!(session.current_referer.is_none());

        session.visit_url("https://example.com/page2".to_string());
        assert_eq!(session.visited_urls.len(), 2);
        assert_eq!(session.current_referer, Some("https://example.com/page1".to_string()));
    }

    #[tokio::test]
    async fn test_intelligent_delay() {
        let config = AdvancedAntiBotConfig::default();

        let start = tokio::time::Instant::now();
        config.intelligent_delay().await;
        let elapsed = start.elapsed();

        // 딜레이가 범위 내에 있는지 확인
        assert!(elapsed.as_secs_f64() >= config.min_delay_secs);
        assert!(elapsed.as_secs_f64() <= config.max_delay_secs + 0.1);  // 여유
    }
}
