import Foundation

/// Host registry for every backend the Yral iOS app talks to.
///
/// Verbatim port of the Kotlin Multiplatform `AppConfigurations`
/// (apps/yral-mobile/shared/core/.../AppConfigurations.kt). Hostnames only —
/// the shared `HttpClientFactory` forces HTTPS for all of these; keep that
/// invariant when constructing URLs in Swift.
public enum YralAppConfiguration {

    /// Legacy upload-AI-video host.
    public static let anonymousIdentityBaseURL = "yral.com"
    /// yral-auth — OAuth/OIDC issuer + JWT minting (self-hosted, Leptos).
    public static let oauthBaseURL = "auth.yral.com"
    /// yral-metadata — username/metadata resolution.
    public static let metadataBaseURL = "metadata.yral.com"
    /// Cloud Run video recommendation service.
    public static let feedBaseURL = "recommendation-service-82502260393.us-central1.run.app"
    /// Anshuman's influencer-feed recsys.
    public static let influencerFeedBaseURL = "recsys-influencer-feed.ansuman.yral.com"
    /// off-chain-agent (report video, rewards config, events bulk).
    public static let offChainBaseURL = "offchain.yral.com"
    /// Unified agent backend (storage interface, videogen, upload, chat, coach).
    /// Prakash's storage-interface was retired 2026-08-21 — all these moved here.
    public static let storageInterfaceBaseURL = "agent.rishi.yral.com"
    public static let videogenBaseURL = "agent.rishi.yral.com"
    public static let uploadBaseURL = "agent.rishi.yral.com"
    public static let chatBaseURL = "agent.rishi.yral.com"
    public static let coachBaseURL = "agent.rishi.yral.com"
    /// Pump/dump game balance (Cloudflare Worker).
    public static let pumpDumpBaseURL = "yral-hot-or-not.go-bazzinga.workers.dev"
    public static let analyticsBaseURL = "analytics.yral.com"
    /// yral-billing — creator earnings + IAP grants.
    public static let billingBaseURL = "billing.sarvesh.yral.com"
    /// yral-daily-streaks.
    public static let dailyStreakBaseURL = "daily-streaks.naitik.yral.com"
    /// Self-hosted Snowplow collector.
    public static let snowplowCollectorURL = "snowplow-collector.yral.com"
    /// SpacetimeDB Maincloud (managed SaaS) — the primary data plane.
    public static let spacetimeDBBaseURL = "maincloud.spacetimedb.com"
    public static let spacetimeDBDatabaseName = "yral-database-spacetime-4lbo7"

    /// True when the given hostname is the yral-auth OAuth host.
    public static func isAuthHost(_ hostname: String) -> Bool {
        hostname == oauthBaseURL
    }
}
