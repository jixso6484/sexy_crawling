use crate::crawler::Crawler;
use crate::error::Result;
use crate::types::{CrawlConfig, Product};
use crate::utils::{AntiBotConfig, build_headers, random_delay_from_config};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, warn};

/// 구글 검색 크롤러 (알리익스프레스 URL 수집용)
pub struct GoogleSearchCrawler {
    client: Client,
    anti_bot_config: AntiBotConfig,
}

impl GoogleSearchCrawler {
    pub fn new() -> Self {
        let anti_bot_config = AntiBotConfig::default();

        let client = Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            anti_bot_config,
        }
    }

    pub fn with_anti_bot_config(anti_bot_config: AntiBotConfig) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            anti_bot_config,
        }
    }

    /// 구글 검색 URL 생성
    ///
    /// 예: https://www.google.com/search?q=site:aliexpress.com+gaming+mouse&start=0
    fn build_search_url(&self, query: &str, page: u32) -> String {
        let start = (page - 1) * 10;  // 구글은 10개씩 표시
        let query_string = format!("site:aliexpress.com {}", query);
        let encoded_query = urlencoding::encode(&query_string);

        format!(
            "https://www.google.com/search?q={}&start={}&num=10",
            encoded_query, start
        )
    }

    /// 구글 검색 결과에서 알리익스프레스 URL 추출
    fn extract_aliexpress_urls(&self, html: &str) -> Result<Vec<String>> {
        let document = Html::parse_document(html);

        let mut urls = Vec::new();

        // 구글 검색 결과 링크 선택자 (여러 가지 시도)
        let selectors = [
            "div.g a[href*='aliexpress.com']",  // 일반 검색 결과
            "a[href*='aliexpress.com'][ping]",  // 추적 링크가 있는 경우
            "cite[role='text']",                // URL 텍스트
            "a[jsname][data-ved]",              // 구글의 내부 링크
        ];

        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    // href 속성에서 URL 추출
                    if let Some(href) = element.value().attr("href") {
                        // 구글 리다이렉트 URL 처리 (/url?q=...)
                        let url = if href.starts_with("/url?q=") {
                            // ?q= 이후의 URL 추출
                            if let Some(start) = href.find("?q=") {
                                let url_part = &href[start + 3..];
                                if let Some(end) = url_part.find('&') {
                                    url_part[..end].to_string()
                                } else {
                                    url_part.to_string()
                                }
                            } else {
                                continue;
                            }
                        } else {
                            href.to_string()
                        };

                        // URL 디코딩
                        if let Ok(decoded_url) = urlencoding::decode(&url) {
                            let url_str = decoded_url.to_string();

                            // 알리익스프레스 URL만 필터링
                            if url_str.contains("aliexpress.com") && !url_str.contains("google.com") {
                                urls.push(url_str);
                            }
                        }
                    }

                    // cite 태그의 경우 텍스트에서 URL 추출
                    if selector_str.contains("cite") {
                        let text = element.text().collect::<String>();
                        if text.contains("aliexpress.com") {
                            // https:// 추가 (cite는 프로토콜 없이 표시)
                            let url = if text.starts_with("https://") {
                                text
                            } else {
                                format!("https://{}", text)
                            };
                            urls.push(url);
                        }
                    }
                }
            }
        }

        // 중복 제거
        urls.sort();
        urls.dedup();

        info!("Extracted {} AliExpress URLs from Google search results", urls.len());

        if urls.is_empty() {
            warn!("No AliExpress URLs found in Google search results");
            warn!("This might indicate that Google is blocking the crawler");
        }

        Ok(urls)
    }

    /// 구글 검색 결과 크롤링 (URL만 수집)
    pub async fn search_urls(&self, query: &str, max_pages: u32) -> Result<Vec<String>> {
        info!("Searching Google for: site:aliexpress.com {}", query);

        let mut all_urls = Vec::new();

        for page in 1..=max_pages {
            let url = self.build_search_url(query, page);
            info!("Fetching Google search results: page {}", page);

            // 봇 차단 방지: 랜덤 딜레이
            if page > 1 {
                random_delay_from_config(&self.anti_bot_config).await;
            }

            // 헤더 설정
            let headers = build_headers(&self.anti_bot_config, None);

            // HTTP 요청
            let response = self.client
                .get(&url)
                .headers(headers)
                .send()
                .await?;

            // CAPTCHA 감지
            let html = response.text().await?;
            if html.contains("unusual traffic") || html.contains("captcha") {
                warn!("Google CAPTCHA detected! Stopping search.");
                break;
            }

            // URL 추출
            let urls = self.extract_aliexpress_urls(&html)?;
            if urls.is_empty() && page > 1 {
                info!("No more results found on page {}", page);
                break;
            }

            all_urls.extend(urls);

            // 너무 많은 페이지를 크롤링하지 않도록 제한
            if page >= 5 {
                warn!("Reached maximum safe page limit (5) for Google search");
                break;
            }
        }

        // 중복 제거
        all_urls.sort();
        all_urls.dedup();

        info!("Total {} unique AliExpress URLs found from Google search", all_urls.len());

        Ok(all_urls)
    }
}

#[async_trait]
impl Crawler for GoogleSearchCrawler {
    fn name(&self) -> &str {
        "Google Search (AliExpress)"
    }

    async fn crawl(&self, _config: &CrawlConfig) -> Result<Vec<Product>> {
        // 구글 검색 크롤러는 URL만 수집하고, 실제 상품 정보는 수집하지 않음
        // 대신 search_urls()를 사용하여 URL 목록을 얻은 후,
        // AliExpressCrawler.extract_product()로 각 URL을 크롤링해야 함

        warn!("GoogleSearchCrawler.crawl() is not recommended for direct product crawling");
        warn!("Use search_urls() to get URLs, then use AliExpressCrawler to crawl each product");

        Ok(Vec::new())
    }

    async fn extract_product(&self, _url: &str) -> Result<Option<Product>> {
        // 구글 검색 크롤러는 개별 상품 페이지를 크롤링하지 않음
        warn!("GoogleSearchCrawler does not support extract_product()");
        warn!("Use AliExpressCrawler.extract_product() instead");

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url() {
        let crawler = GoogleSearchCrawler::new();

        let url = crawler.build_search_url("gaming mouse", 1);
        assert!(url.contains("site:aliexpress.com"));
        assert!(url.contains("gaming"));
        assert!(url.contains("start=0"));

        let url = crawler.build_search_url("gaming mouse", 2);
        assert!(url.contains("start=10"));
    }

    #[tokio::test]
    async fn test_search_urls() {
        // 실제 구글 검색은 테스트하지 않음 (CAPTCHA 위험)
        // 통합 테스트에서 수동으로 확인
    }
}
