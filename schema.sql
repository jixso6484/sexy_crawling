-- SQLite Schema for AliExpress Crawling System
-- 메모리 효율적인 URL 관리 및 크롤링 상태 추적

-- 1. URL 큐 테이블 (방문할 URL 관리)
CREATE TABLE IF NOT EXISTS url_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    url_type TEXT NOT NULL CHECK(url_type IN ('search', 'product', 'category', 'shop')),
    source TEXT NOT NULL CHECK(source IN ('google', 'aliexpress_direct', 'manual')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'processing', 'completed', 'failed', 'skipped')),
    priority INTEGER DEFAULT 0,  -- 높을수록 우선순위 높음
    search_query TEXT,  -- 검색 쿼리 (추적용)
    discovered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    first_attempt_at DATETIME,
    last_attempt_at DATETIME,
    completed_at DATETIME,
    retry_count INTEGER DEFAULT 0,
    error_message TEXT,
    metadata TEXT  -- JSON 형식의 추가 정보
);

CREATE INDEX idx_url_queue_status ON url_queue(status);
CREATE INDEX idx_url_queue_priority ON url_queue(priority DESC, discovered_at ASC);
CREATE INDEX idx_url_queue_source ON url_queue(source);
CREATE INDEX idx_url_queue_type ON url_queue(url_type);

-- 2. 크롤링된 상품 데이터 (메모리 효율을 위해 최소 정보만 저장)
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_url TEXT NOT NULL UNIQUE,
    product_id TEXT,  -- 알리익스프레스 상품 ID
    name TEXT,
    price TEXT,
    original_price TEXT,
    discount_rate TEXT,
    rating REAL,
    review_count INTEGER,
    image_url TEXT,
    seller TEXT,
    source TEXT DEFAULT 'aliexpress',
    crawled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_processed INTEGER DEFAULT 0,  -- Ollama 처리 여부
    raw_data TEXT  -- JSON 형식의 전체 데이터 (필요시)
);

CREATE INDEX idx_products_url ON products(product_url);
CREATE INDEX idx_products_id ON products(product_id);
CREATE INDEX idx_products_is_processed ON products(is_processed);
CREATE INDEX idx_products_crawled_at ON products(crawled_at DESC);

-- 2.1 크롤링된 HTML 캐시 (압축 저장으로 메모리 효율)
CREATE TABLE IF NOT EXISTS html_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    html_content BLOB,  -- gzip 압축된 HTML
    content_hash TEXT,  -- SHA256 해시 (중복 감지)
    content_length INTEGER,  -- 압축 전 크기
    compressed_length INTEGER,  -- 압축 후 크기
    crawled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,  -- 캐시 만료 시간
    is_valid INTEGER DEFAULT 1,  -- 파싱 가능 여부
    error_message TEXT
);

CREATE INDEX idx_html_cache_url ON html_cache(url);
CREATE INDEX idx_html_cache_hash ON html_cache(content_hash);
CREATE INDEX idx_html_cache_expires ON html_cache(expires_at);

-- 3. 크롤링 세션 추적 (성능 모니터링)
CREATE TABLE IF NOT EXISTS crawl_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    crawl_mode TEXT NOT NULL CHECK(crawl_mode IN ('google', 'direct', 'hybrid')),
    search_query TEXT,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    total_urls_discovered INTEGER DEFAULT 0,
    total_urls_crawled INTEGER DEFAULT 0,
    total_products_found INTEGER DEFAULT 0,
    total_errors INTEGER DEFAULT 0,
    status TEXT DEFAULT 'running' CHECK(status IN ('running', 'paused', 'completed', 'failed')),
    config TEXT  -- JSON 형식의 설정 정보
);

CREATE INDEX idx_sessions_status ON crawl_sessions(status);
CREATE INDEX idx_sessions_started ON crawl_sessions(started_at DESC);

-- 4. 크롤링 로그 (에러 추적 및 디버깅)
CREATE TABLE IF NOT EXISTS crawl_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    url TEXT,
    log_level TEXT CHECK(log_level IN ('DEBUG', 'INFO', 'WARN', 'ERROR')),
    message TEXT,
    error_details TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES crawl_sessions(session_id)
);

CREATE INDEX idx_logs_session ON crawl_logs(session_id);
CREATE INDEX idx_logs_level ON crawl_logs(log_level);
CREATE INDEX idx_logs_created ON crawl_logs(created_at DESC);

-- 5. 처리된 상품 데이터 (Ollama 분석 결과)
CREATE TABLE IF NOT EXISTS processed_products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    normalized_name TEXT,
    normalized_price REAL,
    category TEXT,
    features TEXT,  -- JSON array
    summary TEXT,
    processed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

CREATE INDEX idx_processed_product_id ON processed_products(product_id);
CREATE INDEX idx_processed_category ON processed_products(category);

-- 6. 검색 쿼리 추적 (어떤 키워드가 유용한지 분석)
CREATE TABLE IF NOT EXISTS search_queries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL UNIQUE,
    first_searched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_searched_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    search_count INTEGER DEFAULT 1,
    total_results INTEGER DEFAULT 0,
    avg_result_quality REAL  -- 평균 품질 점수 (옵션)
);

CREATE INDEX idx_queries_count ON search_queries(search_count DESC);
CREATE INDEX idx_queries_last ON search_queries(last_searched_at DESC);

-- 뷰: 크롤링 대기 중인 URL (우선순위 순)
CREATE VIEW IF NOT EXISTS pending_urls AS
SELECT
    id,
    url,
    url_type,
    source,
    priority,
    search_query,
    discovered_at,
    retry_count
FROM url_queue
WHERE status = 'pending'
ORDER BY priority DESC, discovered_at ASC;

-- 뷰: 크롤링 통계
CREATE VIEW IF NOT EXISTS crawl_stats AS
SELECT
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_urls,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_urls,
    COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_urls,
    COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_urls,
    (SELECT COUNT(*) FROM products) as total_products,
    (SELECT COUNT(*) FROM products WHERE is_processed = 1) as processed_products,
    (SELECT COUNT(*) FROM crawl_sessions WHERE status = 'running') as active_sessions
FROM url_queue;

-- 뷰: 최근 크롤링된 상품 (성능 모니터링)
CREATE VIEW IF NOT EXISTS recent_products AS
SELECT
    id,
    name,
    price,
    rating,
    review_count,
    source,
    crawled_at,
    is_processed
FROM products
ORDER BY crawled_at DESC
LIMIT 100;
