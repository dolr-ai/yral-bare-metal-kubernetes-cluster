import Foundation

/// Per-test HTTP stubbing at the Apple-native `URLProtocol` seam.
///
/// `URLProtocol` is a class-metatype plugin API: you register the CLASS
/// in `URLSessionConfiguration.protocolClasses` and URLSession
/// instantiates it internally — a per-test handler cannot be injected
/// through the initializer, which is why URLProtocol stubs usually park
/// per-test state in `static var`s. That static state is cross-suite
/// race state under Swift Testing (suites run in parallel; `.serialized`
/// only orders tests within one suite).
///
/// Instead, each test stamps a unique channel id on its session via
/// `URLSessionConfiguration.httpAdditionalHeaders` — per the docs,
/// headers there "are added to all tasks within sessions based on this
/// configuration" — so every request the session makes carries the
/// channel, and `startLoading` routes the request to that test's own
/// handler. No static per-test state, no `.serialized` anywhere.
///
/// FILE SCOPE (not nested in an @MainActor suite): URLProtocol's
/// loading methods run on URLSession's queue; a nested class would
/// inherit MainActor isolation under Swift 6 and trap on executor
/// mismatch (`dispatch_assert_queue` SIGTRAPs — seen live).
final class ChannelURLProtocol: URLProtocol, @unchecked Sendable {

    /// Carries the per-test channel id on every request. A custom
    /// header — never one of the session-managed headers Apple forbids
    /// setting (Authorization/Connection/Host/Proxy-*).
    static let channelHeaderField = "X-Test-Channel"

    private static let registryLock = NSLock()

    /// Per-channel handler registry. The dictionary is process-shared
    /// infrastructure, but every test registers under a fresh UUID key
    /// and never touches another test's channel — behavior is fully
    /// isolated. `nonisolated(unsafe)`: all access is under
    /// `registryLock` (Swift 6 cannot see the lock).
    nonisolated(unsafe) private static var handlersByChannel:
        [String: (@Sendable (URLRequest) throws -> (HTTPURLResponse, Data))] = [:]

    /// Registers `handler` under `channel`. Call any time before the
    /// network request is made (client construction alone is fine).
    static func register(
        _ handler: @escaping @Sendable (URLRequest) throws -> (HTTPURLResponse, Data),
        forChannel channel: String
    ) {
        Self.registryLock.lock()
        defer { Self.registryLock.unlock() }
        Self.handlersByChannel[channel] = handler
    }

    /// Drops the channel's registration — tests call this in `defer` so
    /// a handler never outlives its test.
    static func unregister(channel: String) {
        Self.registryLock.lock()
        defer { Self.registryLock.unlock() }
        Self.handlersByChannel[channel] = nil
    }

    /// An ephemeral, protocol-stubbed session whose every request
    /// carries `channel`.
    static func makeSession(channel: String) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [ChannelURLProtocol.self]
        configuration.httpAdditionalHeaders = [channelHeaderField: channel]
        return URLSession(configuration: configuration)
    }

    static override func canInit(with request: URLRequest) -> Bool { true }
    static override func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        let channel = request.value(forHTTPHeaderField: Self.channelHeaderField) ?? ""
        Self.registryLock.lock()
        let handler = Self.handlersByChannel[channel]
        Self.registryLock.unlock()
        guard let handler else {
            client?.urlProtocol(
                self, didFailWithError: URLError(.unsupportedURL)
            )
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

/// Request recorder — reference-boxed so the @Sendable URLProtocol
/// handler (running off the MainActor) can record into it. File scope
/// for the same isolation reason as `ChannelURLProtocol`.
final class RequestRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var storedRequests: [URLRequest] = []
    private var storedBodies: [String] = []

    var requests: [URLRequest] {
        lock.lock()
        defer { lock.unlock() }
        return storedRequests
    }

    var requestBodies: [String] {
        lock.lock()
        defer { lock.unlock() }
        return storedBodies
    }

    func record(_ request: URLRequest) {
        lock.lock()
        defer { lock.unlock() }
        storedRequests.append(request)
        if let body = request.httpBody ?? request.bodyStreamData {
            storedBodies.append(String(data: body, encoding: .utf8) ?? "")
        }
    }
}

extension URLRequest {
    /// `httpBody` may be nil when the protocol consumed the body stream —
    /// this reads it back for the recording handlers.
    var bodyStreamData: Data? {
        guard let stream = httpBodyStream else { return nil }
        stream.open()
        defer { stream.close() }
        var data = Data()
        let bufferSize = 4_096
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
        defer { buffer.deallocate() }
        while stream.hasBytesAvailable {
            let read = stream.read(buffer, maxLength: bufferSize)
            if read <= 0 { break }
            data.append(buffer, count: read)
        }
        return data
    }
}
