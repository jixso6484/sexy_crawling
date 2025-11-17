# Sexy Crawling 🕷️

쿠팡, 다나와, 알리익스프레스에서 상품 정보를 크롤링하고 Ollama를 사용하여 데이터를 정리하는 Rust 크롤러입니다.

## 주요 기능

- 🛒 **다중 쇼핑몰 지원**: 쿠팡, 다나와, 알리익스프레스
- 🤖 **AI 기반 데이터 처리**: Ollama를 통한 자동 데이터 정규화 및 분석
- ⚡ **비동기 처리**: Tokio 기반 고성능 크롤링
- 📊 **다양한 분석 기능**: 상품 비교, 요약, 정규화
- 🎯 **CLI 인터페이스**: 사용하기 쉬운 명령줄 도구

## 필수 요구사항

1. **Rust** (1.70 이상)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Ollama** (로컬 LLM 서버)
   ```bash
   # macOS/Linux
   curl https://ollama.ai/install.sh | sh

   # Ollama 실행
   ollama serve

   # LLM 모델 다운로드 (예: llama2)
   ollama pull llama2
   ```

## 설치 및 빌드

```bash
# 프로젝트 클론
git clone <repository-url>
cd sexy_crawling

# 빌드
cargo build --release

# 실행 파일 위치
./target/release/sexy_crawling
```

## 사용법

### 1. Ollama 연결 테스트

먼저 Ollama가 정상적으로 작동하는지 확인하세요:

```bash
cargo run -- test
```

### 2. 상품 검색 (크롤링만)

특정 사이트에서 상품 검색:

```bash
# 모든 사이트에서 검색
cargo run -- search "노트북" --output results.json

# 특정 사이트만 검색
cargo run -- search "노트북" --site coupang --output coupang.json
cargo run -- search "노트북" --site danawa --output danawa.json
cargo run -- search "노트북" --site aliexpress --output aliexpress.json

# 최대 페이지 수 지정
cargo run -- search "노트북" --max-pages 5
```

### 3. 크롤링 결과 분석

이미 크롤링된 JSON 파일을 Ollama로 분석:

```bash
# 상품 정보 정규화
cargo run -- analyze results.json --analysis-type process --output processed.json

# 상품 비교
cargo run -- analyze results.json --analysis-type compare

# 상품 요약
cargo run -- analyze results.json --analysis-type summarize
```

### 4. 자동 모드 (크롤링 + 분석)

검색부터 분석까지 한 번에:

```bash
cargo run -- auto "게이밍 마우스" --output full_result.json

# 특정 사이트만
cargo run -- auto "게이밍 마우스" --site coupang
```

### CLI 옵션

```bash
옵션:
  --ollama-url <URL>       Ollama 서버 URL [기본값: http://localhost:11434]
  --ollama-model <MODEL>   사용할 Ollama 모델 [기본값: llama2]
  -m, --max-pages <NUM>    크롤링할 최대 페이지 수 [기본값: 3]
  -v, --verbose            상세 로그 출력
  -h, --help              도움말 표시
```

## 프로젝트 구조

```
sexy_crawling/
├── src/
│   ├── main.rs              # CLI 엔트리포인트
│   ├── lib.rs               # 라이브러리 루트
│   ├── error.rs             # 에러 타입 정의
│   ├── types.rs             # 공통 데이터 타입
│   ├── crawler/
│   │   ├── mod.rs          # 크롤러 trait
│   │   ├── coupang.rs      # 쿠팡 크롤러
│   │   ├── danawa.rs       # 다나와 크롤러
│   │   └── aliexpress.rs   # 알리익스프레스 크롤러
│   └── ollama/
│       ├── mod.rs          # Ollama 모듈
│       ├── client.rs       # Ollama API 클라이언트
│       └── processor.rs    # 데이터 처리 로직
├── Cargo.toml
└── README.md
```

## 사용 예제

### 예제 1: 노트북 가격 비교

```bash
# 1. 모든 사이트에서 노트북 검색
cargo run -- search "맥북" --output laptop.json

# 2. 결과 비교
cargo run -- analyze laptop.json --analysis-type compare
```

### 예제 2: 특정 상품 상세 분석

```bash
# 검색 + 분석을 한 번에
cargo run -- auto "아이폰 15" --output iphone_analysis.json
```

### 예제 3: 커스텀 Ollama 모델 사용

```bash
# 다른 모델 사용 (예: mistral)
ollama pull mistral
cargo run -- --ollama-model mistral auto "게이밍 키보드"
```

## 데이터 구조

### Product (원본 상품 정보)

```json
{
  "name": "삼성 갤럭시 북3 프로 360",
  "price": "1,890,000",
  "original_price": "2,390,000",
  "discount_rate": "21%",
  "rating": 4.8,
  "review_count": 1234,
  "image_url": "https://...",
  "product_url": "https://...",
  "seller": "삼성전자",
  "delivery_info": "무료배송",
  "source": "Coupang"
}
```

### ProcessedProduct (Ollama 처리 후)

```json
{
  "original": { /* Product 객체 */ },
  "normalized_name": "삼성 갤럭시북3 프로 360 13.3인치 i7 16GB 512GB",
  "normalized_price": 1890000.0,
  "category": "노트북/PC",
  "features": [
    "13.3인치",
    "Intel i7",
    "16GB RAM",
    "512GB SSD",
    "2-in-1"
  ],
  "summary": "삼성의 프리미엄 2-in-1 노트북으로 터치스크린과 펜 지원"
}
```

## 주의사항

### 크롤링 관련

1. **사이트 구조 변경**: 쇼핑몰 사이트들은 자주 HTML 구조를 변경합니다. 셀렉터가 작동하지 않으면 업데이트가 필요할 수 있습니다.

2. **요청 제한**: 과도한 요청은 IP 차단을 초래할 수 있습니다. 코드에 내장된 딜레이를 유지하세요.

3. **로봇 차단**: 일부 사이트는 봇을 차단할 수 있습니다. User-Agent를 변경하거나 프록시를 사용해야 할 수 있습니다.

4. **법적 고지**: 웹 크롤링은 사이트의 이용약관을 준수해야 합니다. 상업적 용도로 사용하기 전에 각 사이트의 정책을 확인하세요.

### Ollama 관련

1. **모델 크기**: 큰 모델(예: llama2 70B)은 많은 메모리가 필요합니다. 시스템 사양에 맞는 모델을 선택하세요.

2. **처리 속도**: AI 분석은 시간이 걸립니다. 많은 상품을 처리할 때는 인내심이 필요합니다.

3. **한국어 지원**: 한국어를 잘 지원하는 모델을 사용하세요 (예: llama2, mistral, solar).

## 문제 해결

### Ollama 연결 실패

```bash
# Ollama가 실행 중인지 확인
ps aux | grep ollama

# Ollama 재시작
pkill ollama
ollama serve
```

### 크롤링 결과가 비어있음

- 사이트 구조가 변경되었을 수 있습니다
- `-v` 옵션으로 상세 로그를 확인하세요
- 셀렉터 업데이트가 필요할 수 있습니다

### 빌드 오류

```bash
# 의존성 업데이트
cargo update

# 클린 빌드
cargo clean
cargo build --release
```

## 개발

### 새로운 크롤러 추가

`src/crawler/` 디렉토리에 새 파일을 만들고 `Crawler` trait을 구현하세요:

```rust
use async_trait::async_trait;
use crate::crawler::Crawler;
use crate::types::{CrawlConfig, Product};
use crate::error::Result;

pub struct NewSiteCrawler {
    client: reqwest::Client,
}

#[async_trait]
impl Crawler for NewSiteCrawler {
    fn name(&self) -> &str {
        "NewSite"
    }

    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<Product>> {
        // 구현
        todo!()
    }

    async fn extract_product(&self, url: &str) -> Result<Option<Product>> {
        // 구현
        todo!()
    }
}
```

### 테스트 실행

```bash
cargo test
```

## 기여

이슈와 PR은 언제나 환영합니다!

## 라이선스

MIT License

## 면책 조항

이 도구는 교육 및 개인 사용 목적으로 제작되었습니다. 크롤링하는 웹사이트의 이용약관을 준수할 책임은 사용자에게 있습니다.
