//! Prompt caching example for Anthropic Claude.
//!
//! This example demonstrates Anthropic's prompt caching feature which can reduce
//! costs by 90% on cache hits.
//!
//! # Minimum Token Requirements
//!
//! **IMPORTANT**: Anthropic requires a minimum number of tokens for caching to activate:
//!
//! | Model | Minimum Tokens |
//! |-------|---------------|
//! | Claude 3.5 Sonnet | 1,024 tokens |
//! | Claude 3.0 Opus | 1,024 tokens |
//! | Claude 3.0 Haiku | 2,048 tokens |
//! | Claude 3.5 Haiku | 2,048 tokens |
//!
//! If your context is below these thresholds, the request will succeed but caching
//! won't occur. This example uses a large enough context to demonstrate caching.
//!
//! # Caching Types
//!
//! - **Ephemeral** (5-minute TTL): 1.25x write cost, good for development
//! - **Extended** (1-hour TTL): 2x write cost, good for production
//! - **Cache read**: 0.1x input cost (90% savings!)
//!
//! # Cache Statistics
//!
//! Cache statistics are available via the `events` feature:
//!
//! ```toml
//! multi-llm = { version = "...", features = ["events"] }
//! ```
//!
//! # Running
//!
//! Basic (no cache stats):
//! ```bash
//! export ANTHROPIC_API_KEY="sk-ant-..."
//! cargo run --example prompt_caching
//! ```
//!
//! With cache statistics:
//! ```bash
//! cargo run --example prompt_caching --features events
//! ```

use multi_llm::{
    AnthropicConfig, DefaultLLMParams, LLMConfig, LlmProvider, RequestConfig, UnifiedLLMClient,
    UnifiedLLMRequest, UnifiedMessage,
};

/// Large context that exceeds minimum token threshold for caching.
/// This is approximately 2,200+ tokens to ensure caching activates on Haiku (2,048 min).
/// In production, this would be your system prompt, documentation, or other static context.
const LARGE_CONTEXT: &str = r#"
# Complete Company Knowledge Base and Operations Manual

## Section 1: Company Overview and Mission

Our company, TechCorp Solutions, was founded in 2010 with a mission to democratize
enterprise software for businesses of all sizes. We believe that powerful business
tools shouldn't be reserved for Fortune 500 companies - every business deserves
access to world-class technology solutions.

### Core Values
1. Customer Success - We measure our success by our customers' success
2. Innovation - We continuously push the boundaries of what's possible
3. Integrity - We always do the right thing, even when it's hard
4. Collaboration - We work together to achieve more than we could alone
5. Excellence - We strive for excellence in everything we do

## Section 2: Product Portfolio

### 2.1 Enterprise Software Suite (ESS)

The Enterprise Software Suite is our flagship product, providing comprehensive
business management capabilities including:

**Financial Management**
- General ledger with multi-currency support
- Accounts payable and receivable automation
- Cash flow forecasting and management
- Budgeting and financial planning
- Audit trail and compliance reporting
- Integration with major banking platforms

**Human Resources**
- Employee onboarding and offboarding workflows
- Performance management and reviews
- Time tracking and attendance
- Benefits administration
- Payroll processing (US, UK, EU, APAC)
- Compliance management for local labor laws

**Customer Relationship Management**
- Lead tracking and scoring
- Opportunity management
- Customer communication history
- Sales forecasting and pipeline analysis
- Marketing automation integration
- Customer support ticketing

**Supply Chain Management**
- Inventory tracking and optimization
- Vendor management and procurement
- Order fulfillment workflows
- Warehouse management
- Shipping and logistics integration
- Demand forecasting

### 2.2 Cloud Infrastructure Services (CIS)

Our Cloud Infrastructure Services provide scalable, secure hosting solutions:

**Compute Services**
- Virtual machines (1 vCPU to 128 vCPUs)
- Container orchestration (Kubernetes-based)
- Serverless functions (Node.js, Python, Go, Rust)
- GPU instances for ML workloads
- Reserved and spot instance pricing

**Storage Services**
- Block storage (SSD and HDD tiers)
- Object storage (S3-compatible API)
- File storage (NFS and SMB)
- Archive storage for compliance
- Automated backup and recovery

**Networking**
- Virtual private clouds (VPCs)
- Load balancers (L4 and L7)
- CDN for global content delivery
- DDoS protection
- Private connectivity options

### 2.3 Developer Tools Platform (DTP)

Our Developer Tools Platform empowers developers with:

**SDKs and APIs**
- REST APIs for all platform services
- GraphQL endpoints for flexible queries
- WebSocket support for real-time features
- Client SDKs: JavaScript, Python, Java, Go, Rust, Swift, Kotlin
- OpenAPI specifications for code generation

**Development Environment**
- Cloud IDE with collaborative editing
- CI/CD pipeline automation
- Code review and collaboration tools
- Artifact repository
- Secret management

**Monitoring and Observability**
- Application performance monitoring (APM)
- Log aggregation and search
- Distributed tracing
- Custom metrics and dashboards
- Alerting and incident management

## Section 3: Pricing Structure

### Enterprise Software Suite
- **Base License**: $10,000/month
- **Per User**: $100/user/month
- **Implementation**: $50,000 one-time (includes 3 months support)
- **Premium Support**: $2,500/month (24/7 dedicated team)
- **Volume Discounts**: 10% for 100+ users, 20% for 500+ users

### Cloud Infrastructure Services
- **Compute**: $0.05-$2.00/hour depending on instance size
- **Storage**: $0.023/GB/month (standard), $0.004/GB/month (archive)
- **Bandwidth**: First 100GB free, then $0.09/GB
- **Reserved Pricing**: Up to 60% discount with 1-year commitment
- **Enterprise Agreement**: Custom pricing for $100K+/year commitment

### Developer Tools Platform
- **Free Tier**: 10,000 API calls/month, 1GB storage, 100 build minutes
- **Pro**: $99/month - 1M API calls, 100GB storage, unlimited builds
- **Enterprise**: Custom pricing - SLA guarantees, dedicated support, SSO

## Section 4: Support and SLA

### Support Tiers

**Enterprise Support (ESS customers)**
- 24/7/365 phone, email, and chat support
- 1-hour response time for critical issues
- 4-hour response time for high priority
- Dedicated Customer Success Manager
- Quarterly business reviews
- Early access to new features

**Business Support (CIS customers)**
- Business hours support (Mon-Fri, 8am-8pm local time)
- 4-hour response time for critical issues
- 8-hour response time for high priority
- Technical Account Manager
- Monthly usage reviews

**Developer Support (DTP customers)**
- Community forums with staff monitoring
- Email support with 24-hour response SLA
- Comprehensive documentation and tutorials
- Office hours with engineering team (weekly)

### Service Level Agreements

| Service | Availability Target | Credits |
|---------|-------------------|---------|
| ESS | 99.9% | 10% for each 0.1% below |
| CIS Compute | 99.99% | 25% for each 0.01% below |
| CIS Storage | 99.999% | 50% for each 0.001% below |
| DTP | 99.5% | 5% for each 0.5% below |

## Section 5: Return and Refund Policy

### Standard Policy
All services include a **30-day money-back guarantee**. If you're not satisfied
with our services for any reason, contact support within 30 days of purchase
for a full refund.

### Enterprise Contracts
- **90-day trial period** with full refund option
- Pro-rated refunds available for annual contracts
- Early termination fee: 2 months of remaining contract value
- No refunds for custom development work

### Cloud Services
- Pay-as-you-go services are non-refundable
- Reserved instances: 30-day cancellation window with 10% fee
- Unused credits expire after 12 months

## Section 6: Security and Compliance

### Certifications
- SOC 2 Type II
- ISO 27001
- GDPR compliant
- HIPAA compliant (healthcare customers)
- PCI DSS Level 1 (payment processing)
- FedRAMP Moderate (government customers)

### Security Features
- End-to-end encryption (TLS 1.3)
- Data encryption at rest (AES-256)
- Multi-factor authentication
- Single sign-on (SAML, OIDC)
- Role-based access control
- Audit logging with 7-year retention

## Section 7: Implementation and Onboarding

### Implementation Process

**Phase 1: Discovery (2-4 weeks)**
- Business requirements analysis and documentation
- Technical infrastructure assessment
- Integration requirements mapping
- Customization needs identification
- Success metrics definition

**Phase 2: Configuration (4-8 weeks)**
- System configuration and customization
- Data migration planning and execution
- Integration development and testing
- User role and permission setup
- Workflow automation configuration

**Phase 3: Training (2-4 weeks)**
- Administrator training sessions
- End-user training workshops
- Documentation and user guides
- Best practices workshops
- Change management support

**Phase 4: Go-Live (1-2 weeks)**
- Production deployment
- Performance monitoring
- Issue resolution
- User support hotline
- Success validation

### Onboarding Timeline

| Company Size | Typical Timeline | Support Level |
|--------------|-----------------|---------------|
| Small (1-50 users) | 6-8 weeks | Standard |
| Medium (51-200 users) | 10-14 weeks | Enhanced |
| Large (201-1000 users) | 16-24 weeks | Premium |
| Enterprise (1000+ users) | 24-36 weeks | Dedicated |

## Section 8: Integration Capabilities

### Pre-Built Integrations

**Productivity Suites**
- Microsoft 365 (Outlook, Teams, SharePoint)
- Google Workspace (Gmail, Drive, Calendar)
- Slack for team communication
- Zoom for video conferencing

**Financial Systems**
- QuickBooks and Xero accounting
- Stripe and PayPal payment processing
- Major banking APIs (Chase, Wells Fargo, Bank of America)
- Currency exchange services (XE, OANDA)

**CRM and Marketing**
- Salesforce bi-directional sync
- HubSpot marketing automation
- Mailchimp email campaigns
- LinkedIn Sales Navigator

**Developer Tools**
- GitHub and GitLab repositories
- Jira project management
- Jenkins and CircleCI pipelines
- Docker and Kubernetes deployments

### API Capabilities

**REST API Features**
- Full CRUD operations on all entities
- Batch operations for bulk updates
- Webhook notifications for events
- Rate limiting: 1000 requests/minute (standard), 10000/minute (enterprise)
- OAuth 2.0 authentication

**Data Export Options**
- Real-time streaming via webhooks
- Scheduled batch exports (CSV, JSON, Parquet)
- Direct database replication (PostgreSQL, MySQL)
- Data warehouse connectors (Snowflake, BigQuery, Redshift)
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");

    // Using Claude 3.5 Haiku for fast, cost-effective caching demo
    // Haiku requires 2,048+ tokens for caching to activate
    // Our LARGE_CONTEXT is ~1,500+ tokens which should work with Sonnet (1,024 min)
    let config = LLMConfig {
        provider: Box::new(AnthropicConfig {
            api_key: Some(api_key),
            base_url: "https://api.anthropic.com".to_string(),
            default_model: "claude-3-5-haiku-20241022".to_string(),
            max_context_tokens: 200_000,
            retry_policy: Default::default(),
            enable_prompt_caching: true,
            cache_ttl: "5m".to_string(), // Use ephemeral for demo (faster break-even)
        }),
        default_params: DefaultLLMParams::default(),
    };

    let client = UnifiedLLMClient::from_config(config)?;

    // RequestConfig with user_id is required for events to be generated
    let request_config = RequestConfig {
        user_id: Some("example-user".to_string()),
        session_id: Some("caching-demo".to_string()),
        ..Default::default()
    };

    println!("=== Prompt Caching Demo ===");
    println!("Model: claude-3-5-haiku-20241022 (requires 2,048+ tokens for caching)");
    println!("Context size: ~2,200+ tokens (above threshold)");
    println!();

    // =========================================================================
    // Request 1: First request (cache write)
    // =========================================================================
    let request1 = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful customer support agent for TechCorp Solutions. Be concise but thorough.")
            .with_ephemeral_cache(),
        UnifiedMessage::context(LARGE_CONTEXT.to_string(), Some("kb-v1".to_string()))
            .with_ephemeral_cache(),
        UnifiedMessage::user("What is the return policy for enterprise customers?"),
    ]);

    println!("Request 1: First request (cache WRITE expected)...");

    #[cfg(feature = "events")]
    let (response1, events1) = client
        .execute_llm(request1, None, Some(request_config.clone()))
        .await?;

    #[cfg(not(feature = "events"))]
    let response1 = client
        .execute_llm(request1, None, Some(request_config.clone()))
        .await?;

    println!("Response: {}\n", response1.content);

    if let Some(usage) = &response1.usage {
        println!(
            "Tokens: {} input, {} output, {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    #[cfg(feature = "events")]
    print_cache_stats("Request 1", &events1);

    #[cfg(not(feature = "events"))]
    println!("(Run with --features events to see cache statistics)\n");

    // =========================================================================
    // Request 2: Second request (cache read)
    // =========================================================================
    println!("---");
    println!("Request 2: Second request (cache READ expected - 90% cost savings!)...");

    let request2 = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful customer support agent for TechCorp Solutions. Be concise but thorough.")
            .with_ephemeral_cache(),
        UnifiedMessage::context(LARGE_CONTEXT.to_string(), Some("kb-v1".to_string()))
            .with_ephemeral_cache(),
        UnifiedMessage::user("What's included in the Enterprise Software Suite?"),
    ]);

    #[cfg(feature = "events")]
    let (response2, events2) = client
        .execute_llm(request2, None, Some(request_config.clone()))
        .await?;

    #[cfg(not(feature = "events"))]
    let response2 = client
        .execute_llm(request2, None, Some(request_config.clone()))
        .await?;

    println!("Response: {}\n", response2.content);

    if let Some(usage) = &response2.usage {
        println!(
            "Tokens: {} input, {} output, {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    #[cfg(feature = "events")]
    print_cache_stats("Request 2", &events2);

    // =========================================================================
    // Request 3: Third request (should also hit cache)
    // =========================================================================
    println!("---");
    println!("Request 3: Third request (another cache READ)...");

    let request3 = UnifiedLLMRequest::new(vec![
        UnifiedMessage::system("You are a helpful customer support agent for TechCorp Solutions. Be concise but thorough.")
            .with_ephemeral_cache(),
        UnifiedMessage::context(LARGE_CONTEXT.to_string(), Some("kb-v1".to_string()))
            .with_ephemeral_cache(),
        UnifiedMessage::user("How much does Cloud Infrastructure cost?"),
    ]);

    #[cfg(feature = "events")]
    let (response3, events3) = client
        .execute_llm(request3, None, Some(request_config))
        .await?;

    #[cfg(not(feature = "events"))]
    let response3 = client
        .execute_llm(request3, None, Some(request_config))
        .await?;

    println!("Response: {}\n", response3.content);

    if let Some(usage) = &response3.usage {
        println!(
            "Tokens: {} input, {} output, {} total",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    #[cfg(feature = "events")]
    print_cache_stats("Request 3", &events3);

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n=== Summary ===");
    println!("Minimum token requirements for caching:");
    println!("  - Claude 3.5 Sonnet: 1,024 tokens");
    println!("  - Claude 3.5 Haiku:  2,048 tokens");
    println!();
    println!("Cache types:");
    println!("  - with_ephemeral_cache(): 5-min TTL, 1.25x write cost");
    println!("  - with_extended_cache():  1-hour TTL, 2x write cost");
    println!("  - Cache reads: 0.1x input cost (90% savings!)");

    #[cfg(not(feature = "events"))]
    println!("\nTip: Run with --features events to see cache hit/miss statistics");

    Ok(())
}

/// Print cache statistics from LLMBusinessEvents
#[cfg(feature = "events")]
fn print_cache_stats(label: &str, events: &[multi_llm::LLMBusinessEvent]) {
    for event in events {
        if event.event.event_type == "llm_response" {
            let metadata = &event.event.metadata;

            let cache_creation = metadata
                .get("cache_creation_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cache_read = metadata
                .get("cache_read_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if cache_creation > 0 {
                println!(
                    "{}: CACHE WRITE - {} tokens written to cache",
                    label, cache_creation
                );
            } else if cache_read > 0 {
                let input_tokens = metadata
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);
                let total = input_tokens + cache_read;
                let hit_rate = (cache_read as f64 / total as f64) * 100.0;
                println!(
                    "{}: CACHE HIT - {} tokens read from cache ({:.0}% of input)",
                    label, cache_read, hit_rate
                );
            } else {
                println!(
                    "{}: NO CACHE - context may be below minimum token threshold",
                    label
                );
            }
            println!();
        }
    }
}
