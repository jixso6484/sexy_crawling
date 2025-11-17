# CAPTCHA 회피 전략 가이드

## ⚠️  면책 조항
이 문서는 **교육 목적**과 **합법적인 크롤링 프로젝트**를 위한 것입니다.
CAPTCHA는 웹사이트 보안의 중요한 부분이며, 무단 우회는 법적 문제를 야기할 수 있습니다.
항상 해당 웹사이트의 이용 약관과 robots.txt를 확인하세요.

---

## 🎯 CAPTCHA 유형 및 대응 전략

### 1. **reCAPTCHA v2 (이미지 선택)**
```
특징:
- "자전거가 있는 이미지를 모두 선택하세요"
- 9~16개의 이미지 그리드
- 클릭 패턴 분석

난이도: 중간
```

**대응 전략:**
1. **봇 탐지 우회 (우선)** - reCAPTCHA가 나타나지 않게 함
2. **2Captcha/Anti-Captcha 서비스** - 사람이 직접 풀어줌
3. **머신러닝 모델** - YOLO, ResNet 등으로 이미지 인식
4. **Undetected ChromeDriver** - 봇 감지 회피

### 2. **reCAPTCHA v3 (스코어 기반)**
```
특징:
- 사용자에게 보이지 않음
- 0.0~1.0 점수 (0.0 = 봇, 1.0 = 사람)
- 행동 패턴 분석

난이도: 높음
```

**대응 전략:**
1. **완벽한 브라우저 시뮬레이션** ✅ (현재 구현 중)
2. **자연스러운 행동 패턴** ✅ (구현됨)
3. **세션 일관성 유지** ✅ (구현됨)
4. **마우스 움직임 시뮬레이션** (Headless Browser 필요)

### 3. **hCaptcha**
```
특징:
- reCAPTCHA 대안
- 이미지 선택 방식
- 접근성 기능 (음성 CAPTCHA)

난이도: 중간~높음
```

**대응 전략:**
1. **2Captcha/Anti-Captcha** (지원됨)
2. **봇 탐지 우회** (reCAPTCHA v2와 유사)
3. **음성 CAPTCHA 우회** (음성 인식 API)

### 4. **Cloudflare Turnstile**
```
특징:
- 매우 빠름 (보이지 않는 경우 많음)
- 브라우저 핑거프린트 기반
- JavaScript Challenge

난이도: 중간
```

**대응 전략:**
1. **TLS 핑거프린트 일치** (curl-impersonate)
2. **JavaScript Challenge 해결** (Headless Browser)
3. **쿠키 재사용** (이전 세션 유지)

### 5. **FunCaptcha (Arkose Labs)**
```
특징:
- 게임 형태 (물체 회전, 퍼즐)
- 매우 어려움

난이도: 매우 높음
```

**대응 전략:**
1. **봇 탐지 우회만 가능** (실제 우회는 거의 불가능)
2. **2Captcha 서비스** (고가)
3. **사람 개입** (수동 해결)

---

## ✅ 권장 전략 (난이도별)

### 🟢 Level 1: 봇 탐지 예방 (가장 효과적) ✅
**목표:** CAPTCHA가 나타나지 않게 하기

```rust
// 현재 구현된 기능들:
✅ 브라우저 프로필 시스템
✅ 실제 브라우저 헤더
✅ 가우시안 분포 딜레이
✅ 세션 일관성 유지
✅ Referer 체인 관리
✅ 쿠키 저장

// 추가 필요:
⚠️  프록시 로테이션
⚠️  더 정교한 타이밍
⚠️  자산(CSS/JS/이미지) 요청 시뮬레이션
```

**효과:** 70-80% CAPTCHA 출현 방지

### 🟡 Level 2: CAPTCHA 솔버 서비스 (합법적)
**목표:** 나타난 CAPTCHA를 사람이 풀어줌

```rust
// 2Captcha API 예시
use reqwest::Client;

async fn solve_recaptcha(
    site_key: &str,
    page_url: &str,
    api_key: &str,
) -> Result<String> {
    let client = Client::new();

    // 1. CAPTCHA 제출
    let response = client
        .post("https://2captcha.com/in.php")
        .form(&[
            ("key", api_key),
            ("method", "userrecaptcha"),
            ("googlekey", site_key),
            ("pageurl", page_url),
        ])
        .send()
        .await?
        .text()
        .await?;

    // 2. 결과 폴링 (20~60초 소요)
    let captcha_id = response.split('|').nth(1).unwrap();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let result = client
            .get("https://2captcha.com/res.php")
            .query(&[
                ("key", api_key),
                ("action", "get"),
                ("id", captcha_id),
            ])
            .send()
            .await?
            .text()
            .await?;

        if result.starts_with("OK") {
            return Ok(result.split('|').nth(1).unwrap().to_string());
        }
    }
}
```

**비용:**
- 2Captcha: $2.99/1000 reCAPTCHA v2
- Anti-Captcha: $1.50/1000 reCAPTCHA v2
- CapSolver: $0.80/1000 reCAPTCHA v2

**장점:**
- ✅ 합법적
- ✅ 거의 100% 성공률
- ✅ 구현 간단

**단점:**
- ❌ 비용 발생
- ❌ 느림 (20~60초/CAPTCHA)
- ❌ 외부 서비스 의존

### 🔴 Level 3: Headless Browser (고급)
**목표:** 실제 브라우저처럼 완벽하게 시뮬레이션

```rust
// undetected-chromedriver (Python) 예시
from selenium import webdriver
import undetected_chromedriver as uc

# Rust에서는 thirtyfour 사용 가능
async fn create_undetected_browser() -> WebDriver {
    let caps = DesiredCapabilities::chrome();
    caps.add_chrome_option("--disable-blink-features=AutomationControlled");
    caps.add_chrome_option("--disable-dev-shm-usage");

    WebDriver::new("http://localhost:9515", caps).await?
}
```

**장점:**
- ✅ reCAPTCHA v3 우회 가능
- ✅ JavaScript 실행 가능
- ✅ 완벽한 브라우저 시뮬레이션

**단점:**
- ❌ 느림 (메모리/CPU 많이 사용)
- ❌ 복잡함
- ❌ 여전히 탐지 가능

---

## 🛠️  구현 계획

### Phase 1: 봇 탐지 예방 강화 (현재) ✅
```
✅ 브라우저 프로필 시스템
✅ 고급 헤더 생성
✅ 가우시안 분포 딜레이
✅ 세션 관리
✅ Referer 체인
```

### Phase 2: CAPTCHA 솔버 통합 (다음)
```rust
// src/captcha/solver.rs 구현 예정

pub struct CaptchaSolver {
    api_key: String,
    service: SolverService,  // TwoCaptcha, AntiCaptcha, etc.
}

impl CaptchaSolver {
    pub async fn solve_recaptcha_v2(&self, site_key: &str, url: &str) -> Result<String>;
    pub async fn solve_hcaptcha(&self, site_key: &str, url: &str) -> Result<String>;
}
```

### Phase 3: Headless Browser 통합 (선택적)
```rust
// src/browser/undetected.rs 구현 예정

pub struct UndetectedBrowser {
    driver: WebDriver,
}

impl UndetectedBrowser {
    pub async fn new() -> Result<Self>;
    pub async fn get(&mut self, url: &str) -> Result<()>;
    pub async fn solve_captcha(&mut self) -> Result<()>;
}
```

---

## 📊 전략 비교표

| 전략 | 비용 | 속도 | 성공률 | 난이도 | 권장도 |
|------|------|------|--------|--------|--------|
| **봇 탐지 예방** | 무료 | 빠름 | 70-80% | 중간 | ⭐⭐⭐⭐⭐ |
| **프록시 로테이션** | $20-100/월 | 빠름 | +10-15% | 쉬움 | ⭐⭐⭐⭐ |
| **CAPTCHA 솔버** | $1-3/1000 | 느림 | 95-99% | 쉬움 | ⭐⭐⭐⭐ |
| **Headless Browser** | 무료 | 매우 느림 | 85-95% | 어려움 | ⭐⭐⭐ |
| **ML 기반 인식** | 무료 | 중간 | 60-70% | 매우 어려움 | ⭐⭐ |

---

## 🎓 사이트별 CAPTCHA 전략

### 알리익스프레스
- **CAPTCHA 유형:** reCAPTCHA v2 (가끔)
- **트리거:** 빈번한 요청, 봇 행동 패턴
- **권장 전략:**
  1. 현재 봇 탐지 예방 (70% 효과)
  2. 프록시 로테이션 추가 (+15%)
  3. CAPTCHA 솔버 준비 (최후의 수단)

### 아마존
- **CAPTCHA 유형:** reCAPTCHA v2/v3, Cloudflare
- **트리거:** IP 평판, 요청 빈도
- **권장 전략:**
  1. Headless Browser + Proxy
  2. CAPTCHA 솔버 필수
  3. 매우 느린 크롤링 (1분/페이지)

### 구글
- **CAPTCHA 유형:** reCAPTCHA v3 (고급)
- **트리거:** 모든 자동화 감지
- **권장 전략:**
  1. 공식 API 사용 (Google Custom Search)
  2. Residential Proxy + Headless Browser
  3. 사람 행동 완벽 시뮬레이션

---

## 💡 실전 팁

### 1. CAPTCHA 트리거 최소화
```rust
// ✅ 좋은 예
- 요청 간격: 2-5초 (가우시안 분포)
- 세션 유지: 10-30분
- 여러 페이지 방문: 메인 → 카테고리 → 상품
- 프록시 로테이션: 100-500 요청마다

// ❌ 나쁜 예
- 요청 간격: 정확히 1초마다
- 세션: 매번 새로 생성
- 직접 상품 페이지로 이동
- 동일 IP로 수천 건 요청
```

### 2. CAPTCHA 발생 시 대응
```rust
async fn handle_captcha_if_present(html: &str) -> Result<Option<String>> {
    // 1. CAPTCHA 감지
    if html.contains("g-recaptcha") || html.contains("cf-challenge") {
        warn!("CAPTCHA detected!");

        // 2. 세션 중단 및 딜레이
        tokio::time::sleep(Duration::from_secs(300)).await;  // 5분 대기

        // 3. IP 변경 (프록시 로테이션)
        rotate_proxy().await?;

        // 4. 재시도 또는 CAPTCHA 솔버 호출
        let solution = solve_captcha_with_2captcha().await?;

        return Ok(Some(solution));
    }

    Ok(None)
}
```

### 3. 성공률 모니터링
```rust
struct CaptchaStats {
    total_requests: u64,
    captcha_encounters: u64,
    captcha_solved: u64,
    failed: u64,
}

impl CaptchaStats {
    fn success_rate(&self) -> f64 {
        1.0 - (self.captcha_encounters as f64 / self.total_requests as f64)
    }

    fn solve_rate(&self) -> f64 {
        self.captcha_solved as f64 / self.captcha_encounters as f64
    }
}
```

---

## 🔐 보안 및 윤리

### ✅ 허용되는 경우
- 공개 데이터 수집 (가격, 리뷰 등)
- 웹사이트가 크롤링을 명시적으로 허용
- 교육/연구 목적
- robots.txt 준수

### ❌ 금지되는 경우
- 로그인이 필요한 개인 정보
- 저작권이 있는 콘텐츠 대량 다운로드
- DDoS 공격 의도
- 웹사이트 이용 약관 위반

---

## 📚 추가 리소스

- [2Captcha API 문서](https://2captcha.com/api-docs)
- [Anti-Captcha](https://anti-captcha.com/)
- [undetected-chromedriver](https://github.com/ultrafunkamsterdam/undetected-chromedriver)
- [curl-impersonate](https://github.com/lwthiker/curl-impersonate)
- [reCAPTCHA v3 점수 개선](https://developers.google.com/recaptcha/docs/v3)

---

## 🎯 결론

**최선의 전략:** CAPTCHA가 나타나지 않도록 예방하기

우리의 현재 구현(브라우저 프로필, 지능형 딜레이, 세션 관리)으로
대부분의 경우 CAPTCHA를 피할 수 있습니다.

**다음 단계:**
1. ✅ 봇 탐지 예방 (구현 완료)
2. 🔄 프록시 로테이션 (구현 예정)
3. 🔄 CAPTCHA 솔버 통합 (선택적)
4. 🔄 Headless Browser (고급 사용자용)

**비용 대비 효과:**
- 봇 탐지 예방: $0, 70-80% 성공
- + 프록시: $50/월, 85-90% 성공
- + CAPTCHA 솔버: +$10-50/월, 95-99% 성공
