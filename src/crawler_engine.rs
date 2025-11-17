use crate::crawler::{Crawler, aliexpress::AliExpressCrawler, google::GoogleSearchCrawler};
use crate::db::{Database, url_queue::*, html_cache::HtmlCache, products::ProductManager, sessions::*};
use crate::utils::{HumanBehavior, AntiBotConfig, RateLimiter};
use crate::types::{CrawlConfig, Product};
use anyhow::Result;
use tracing::{info, warn, error, debug};
use std::sync::Arc;

/// 크롤링 엔진 설정
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrawlerEngineConfig {
    pub max_concurrent_crawls: usize,  // 동시 크롤링 수
    pub crawl_mode: CrawlMode,         // 크롤링 모드
    pub human_behavior: HumanBehavior, // 사람 행동 시뮬레이션
    pub anti_bot: AntiBotConfig,       // 봇 차단 방지
    pub save_html: bool,               // HTML 저장 여부
    pub html_cache_hours: i64,         // HTML 캐시 유효 시간
    pub max_retries: u32,              // URL당 최대 재시도
}

impl Default for CrawlerEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_crawls: 1,  // 안전을 위해 기본은 1개씩
            crawl_mode: CrawlMode::Hybrid,
            human_behavior: HumanBehavior::default(),
            anti_bot: AntiBotConfig::default(),
            save_html: true,
            html_cache_hours: 24,
            max_retries: 3,
        }
    }
}

/// 크롤링 엔진 (지속적 크롤링 관리)
pub struct CrawlerEngine {
    db: Arc<Database>,
    config: CrawlerEngineConfig,
    rate_limiter: RateLimiter,
    session_id: Option<String>,
}

impl CrawlerEngine {
    pub fn new(db: Database, config: CrawlerEngineConfig) -> Self {
        let rate_limiter = RateLimiter::new(1500);  // 기본 1.5초 간격

        Self {
            db: Arc::new(db),
            config,
            rate_limiter,
            session_id: None,
        }
    }

    /// 새 세션 시작
    pub async fn start_session(&mut self, search_query: Option<&str>) -> Result<String> {
        let session_manager = SessionManager::new(self.db.pool().clone());

        let config_json = serde_json::to_string(&self.config).ok();

        let session_id = session_manager
            .start_session(self.config.crawl_mode.clone(), search_query, config_json.as_deref())
            .await?;

        self.session_id = Some(session_id.clone());
        info!("Started new crawling session: {}", session_id);

        Ok(session_id)
    }

    /// 세션 종료
    pub async fn finish_session(&self, status: SessionStatus) -> Result<()> {
        if let Some(session_id) = &self.session_id {
            let session_manager = SessionManager::new(self.db.pool().clone());
            session_manager.finish_session(session_id, status).await?;
            info!("Finished session: {}", session_id);
        }

        Ok(())
    }

    /// 구글 검색으로 URL 수집
    pub async fn discover_urls_from_google(&self, query: &str, max_pages: u32) -> Result<usize> {
        info!("Discovering URLs from Google for query: {}", query);

        let google_crawler = GoogleSearchCrawler::with_anti_bot_config(self.config.anti_bot.clone());

        // 구글 검색 실행
        let urls = google_crawler.search_urls(query, max_pages).await?;

        if urls.is_empty() {
            warn!("No URLs discovered from Google search");
            return Ok(0);
        }

        // SQLite에 URL 추가
        let url_queue = UrlQueue::new(self.db.pool().clone());

        let url_data: Vec<_> = urls
            .into_iter()
            .map(|url| {
                (
                    url,
                    UrlType::Product,
                    UrlSource::Google,
                    5,  // 구글에서 발견한 URL은 우선순위 높음
                    Some(query.to_string()),
                )
            })
            .collect();

        let added = url_queue.add_urls_batch(&url_data).await?;

        info!("Added {} URLs to queue from Google search", added);

        // 세션 통계 업데이트
        if let Some(session_id) = &self.session_id {
            let session_manager = SessionManager::new(self.db.pool().clone());
            session_manager
                .update_session_stats(session_id, added as i64, 0, 0, 0)
                .await?;
        }

        Ok(added)
    }

    /// 알리익스프레스 직접 검색으로 URL 수집
    pub async fn discover_urls_from_aliexpress(&self, query: &str, max_pages: u32) -> Result<usize> {
        info!("Discovering URLs from AliExpress direct search for query: {}", query);

        // 알리익스프레스 크롤러 생성
        let user_agent = crate::utils::get_random_user_agent();
        let crawler = AliExpressCrawler::new(user_agent)?;

        // 크롤링 설정
        let crawl_config = CrawlConfig {
            search_query: query.to_string(),
            max_pages,
            timeout_secs: 30,
            user_agent: crate::utils::get_random_user_agent().to_string(),
        };

        // 상품 크롤링
        let products = crawler.crawl(&crawl_config).await?;

        if products.is_empty() {
            warn!("No products discovered from AliExpress direct search");
            return Ok(0);
        }

        // URL 수집
        let urls: Vec<String> = products
            .iter()
            .map(|p| p.product_url.clone())
            .collect();

        // SQLite에 URL 추가
        let url_queue = UrlQueue::new(self.db.pool().clone());

        let url_data: Vec<_> = urls
            .into_iter()
            .map(|url| {
                (
                    url,
                    UrlType::Product,
                    UrlSource::AliexpressDirect,
                    3,  // 직접 검색 URL은 중간 우선순위
                    Some(query.to_string()),
                )
            })
            .collect();

        let added = url_queue.add_urls_batch(&url_data).await?;

        // 상품 데이터도 저장
        let product_manager = ProductManager::new(self.db.pool().clone());
        let saved = product_manager.save_products_batch(&products).await?;

        info!("Added {} URLs and saved {} products from AliExpress", added, saved);

        // 세션 통계 업데이트
        if let Some(session_id) = &self.session_id {
            let session_manager = SessionManager::new(self.db.pool().clone());
            session_manager
                .update_session_stats(session_id, added as i64, 0, saved as i64, 0)
                .await?;
        }

        Ok(added)
    }

    /// 하이브리드 모드: 구글 + 알리익스프레스
    pub async fn discover_urls_hybrid(&self, query: &str, max_pages: u32) -> Result<usize> {
        info!("Discovering URLs using hybrid mode (Google + AliExpress)");

        // 먼저 구글 검색
        let google_count = self.discover_urls_from_google(query, max_pages.min(3)).await?;

        // 사람처럼 딜레이
        self.config.human_behavior.simulate_page_visit().await;

        // 알리익스프레스 직접 검색
        let aliexpress_count = self.discover_urls_from_aliexpress(query, max_pages).await?;

        Ok(google_count + aliexpress_count)
    }

    /// 단일 URL 크롤링
    async fn crawl_single_url(&self, url_item: &UrlQueueItem) -> Result<Option<Product>> {
        info!("Crawling URL: {}", url_item.url);

        // Rate limiting
        self.rate_limiter.wait_for_url(&url_item.url).await;

        // HTML 캐시 확인
        let html_cache = HtmlCache::new(self.db.pool().clone());

        if let Some(_cached_html) = html_cache.get_html(&url_item.url).await? {
            debug!("Using cached HTML for: {}", url_item.url);
            // 캐시된 HTML로 파싱
            // TODO: HTML에서 직접 파싱하는 메서드 추가
        }

        // 사람 행동 시뮬레이션
        self.config.human_behavior.simulate_navigation().await;

        // 크롤러로 상품 추출
        let user_agent = crate::utils::get_random_user_agent();
        let crawler = AliExpressCrawler::new(user_agent)?;

        let product = crawler.extract_product(&url_item.url).await?;

        // HTML 저장 (옵션)
        if self.config.save_html && product.is_some() {
            // TODO: HTML 저장 로직
        }

        // 사람처럼 페이지 읽기
        self.config.human_behavior.simulate_page_reading().await;

        Ok(product)
    }

    /// 대기 중인 URL 크롤링 (메인 루프)
    pub async fn crawl_pending_urls(&self, max_urls: usize) -> Result<usize> {
        info!("Starting to crawl pending URLs (max: {})", max_urls);

        let url_queue = UrlQueue::new(self.db.pool().clone());
        let product_manager = ProductManager::new(self.db.pool().clone());

        // 대기 중인 URL 가져오기
        let pending_urls = url_queue.get_next_urls(max_urls as i64).await?;

        if pending_urls.is_empty() {
            info!("No pending URLs to crawl");
            return Ok(0);
        }

        info!("Found {} pending URLs", pending_urls.len());

        let mut crawled = 0;
        let mut errors = 0;

        for url_item in pending_urls {
            // 처리 중으로 표시
            url_queue
                .update_status(url_item.id, UrlStatus::Processing, None)
                .await?;

            // 크롤링 시도
            match self.crawl_single_url(&url_item).await {
                Ok(Some(product)) => {
                    // 상품 저장
                    if let Err(e) = product_manager.save_product(&product).await {
                        warn!("Failed to save product: {}", e);
                    }

                    // 완료로 표시
                    url_queue
                        .update_status(url_item.id, UrlStatus::Completed, None)
                        .await?;

                    crawled += 1;
                    info!("Successfully crawled: {}", url_item.url);
                }
                Ok(None) => {
                    warn!("No product found at: {}", url_item.url);

                    url_queue
                        .update_status(url_item.id, UrlStatus::Skipped, Some("No product found"))
                        .await?;
                }
                Err(e) => {
                    error!("Failed to crawl {}: {}", url_item.url, e);

                    url_queue
                        .update_status(url_item.id, UrlStatus::Failed, Some(&e.to_string()))
                        .await?;

                    errors += 1;
                }
            }

            // 사람처럼 랜덤 휴식
            self.config.human_behavior.maybe_take_break().await;
        }

        // 세션 통계 업데이트
        if let Some(session_id) = &self.session_id {
            let session_manager = SessionManager::new(self.db.pool().clone());
            session_manager
                .update_session_stats(session_id, 0, crawled as i64, crawled as i64, errors as i64)
                .await?;
        }

        info!("Crawling completed: {} successful, {} errors", crawled, errors);

        Ok(crawled)
    }

    /// 지속적 크롤링 (무한 루프)
    pub async fn run_continuous(&mut self, query: &str, discover_interval_mins: u64) -> Result<()> {
        info!("Starting continuous crawling for query: {}", query);

        // 세션 시작
        self.start_session(Some(query)).await?;

        let mut iteration = 0;

        loop {
            iteration += 1;
            info!("=== Crawling iteration {} ===", iteration);

            // 1. URL 발견 (주기적으로)
            if iteration % (discover_interval_mins as usize) == 0 || iteration == 1 {
                match self.config.crawl_mode {
                    CrawlMode::Google => {
                        self.discover_urls_from_google(query, 5).await?;
                    }
                    CrawlMode::Direct => {
                        self.discover_urls_from_aliexpress(query, 5).await?;
                    }
                    CrawlMode::Hybrid => {
                        self.discover_urls_hybrid(query, 5).await?;
                    }
                }
            }

            // 2. 대기 중인 URL 크롤링
            let crawled = self.crawl_pending_urls(10).await?;

            if crawled == 0 {
                info!("No URLs to crawl, waiting for next discovery...");
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }

            // 3. 통계 출력
            let stats = self.db.get_stats().await?;
            stats.print();

            // 4. 캐시 정리 (주기적으로)
            if iteration % 100 == 0 {
                self.db.cleanup(7).await?;  // 7일 이상 된 캐시 삭제
            }

            // 종료 조건 확인 (예: 모든 URL 처리 완료)
            if stats.pending_urls == 0 && crawled == 0 {
                info!("All URLs processed. Continuous crawling complete.");
                break;
            }
        }

        // 세션 종료
        self.finish_session(SessionStatus::Completed).await?;

        Ok(())
    }

    /// 통계 출력
    pub async fn print_stats(&self) -> Result<()> {
        let stats = self.db.get_stats().await?;
        stats.print();

        let url_queue = UrlQueue::new(self.db.pool().clone());
        let queue_stats = url_queue.get_queue_stats().await?;
        queue_stats.print();

        let product_manager = ProductManager::new(self.db.pool().clone());
        let product_stats = product_manager.get_product_stats().await?;
        product_stats.print();

        let html_cache = HtmlCache::new(self.db.pool().clone());
        let cache_stats = html_cache.get_cache_stats().await?;
        cache_stats.print();

        Ok(())
    }
}
