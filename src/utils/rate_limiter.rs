use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// 도메인별 Rate Limiter
#[derive(Clone)]
pub struct RateLimiter {
    last_request: Arc<Mutex<HashMap<String, Instant>>>,
    min_interval: Duration,
}

impl RateLimiter {
    /// 새 RateLimiter 생성
    ///
    /// # Arguments
    /// * `min_interval_ms` - 최소 요청 간격 (밀리초)
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            last_request: Arc::new(Mutex::new(HashMap::new())),
            min_interval: Duration::from_millis(min_interval_ms),
        }
    }

    /// 요청 전 대기 (필요시)
    ///
    /// # Arguments
    /// * `domain` - 도메인 (예: "aliexpress.com")
    pub async fn wait(&self, domain: &str) {
        let mut last_requests = self.last_request.lock().await;

        if let Some(&last_time) = last_requests.get(domain) {
            let elapsed = last_time.elapsed();

            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                drop(last_requests);  // 락 해제

                tracing::debug!(
                    "Rate limiting: waiting {:?} for {}",
                    wait_time,
                    domain
                );

                tokio::time::sleep(wait_time).await;

                let mut last_requests = self.last_request.lock().await;
                last_requests.insert(domain.to_string(), Instant::now());
            } else {
                last_requests.insert(domain.to_string(), Instant::now());
            }
        } else {
            last_requests.insert(domain.to_string(), Instant::now());
        }
    }

    /// URL에서 도메인 추출
    pub fn extract_domain(url: &str) -> Option<String> {
        let url = url::Url::parse(url).ok()?;
        url.host_str().map(|s| s.to_string())
    }

    /// URL 기반 대기
    pub async fn wait_for_url(&self, url: &str) {
        if let Some(domain) = Self::extract_domain(url) {
            self.wait(&domain).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        let domain = RateLimiter::extract_domain("https://www.aliexpress.com/products/123");
        assert_eq!(domain, Some("www.aliexpress.com".to_string()));

        let domain = RateLimiter::extract_domain("https://google.com/search?q=test");
        assert_eq!(domain, Some("google.com".to_string()));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(1000);  // 1초 간격

        let start = Instant::now();
        limiter.wait("test.com").await;
        let first_elapsed = start.elapsed();

        limiter.wait("test.com").await;
        let second_elapsed = start.elapsed();

        // 두 번째 요청은 최소 1초 이상 걸려야 함
        assert!(second_elapsed.as_millis() >= 1000);

        // 첫 번째 요청은 즉시
        assert!(first_elapsed.as_millis() < 100);
    }
}
