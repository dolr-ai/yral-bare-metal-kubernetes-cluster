import SwiftUI

#if canImport(UIKit)
import AudioToolbox
import UIKit

/// GPU-accelerated confetti — SwiftUI's native Canvas + TimelineView
/// (the modern replacement for the particle-emitter dance; Canvas draws
/// via Metal, so hundreds of flakes cost nothing — no per-frame Swift
/// object churn, no stutters). One deterministic burst at the done step.
///
/// The party horn plays in sync (system sound 1104, AudioServices — no
/// audio session juggling for a one-shot effect).
struct CelebrationView: View {
    /// Confetti flakes, seeded once — pure value structs, updated by
    /// elapsed time inside the Canvas draw (no @State churn per frame).
    private let flakes: [ConfettiFlake]

    /// The system "party horn" sound. 1104 is the short celebratory
    /// horn; played once at creation. NOT muted in previews — the
    /// operator wants to hear exactly what ships.
    private let playsPartyHorn: Bool

    init(playsPartyHorn: Bool = true) {
        self.playsPartyHorn = playsPartyHorn
        // Deterministic pseudo-random spread from a fixed seed — same
        // celebration every time, and previewable.
        var seed: UInt64 = 0x2026_0901
        func nextRandom() -> Double {
            seed = seed &* 6364136223846793005 &+ 1442695040888963407
            return Double((seed >> 33) & 0xFFFF) / 65_535.0
        }
        var flakes: [ConfettiFlake] = []
        for index in 0..<ConfettiFlake.flakeCount {
            flakes.append(
                ConfettiFlake(
                    index: index,
                    startX: nextRandom(),
                    startY: nextRandom(),
                    horizontalDrift: (nextRandom() - 0.5) * 0.25,
                    fallSpeed: 0.28 + nextRandom() * 0.22,
                    rotationSpeed: (nextRandom() - 0.5) * 8.0,
                    colorIndex: index % ConfettiFlake.palette.count
                )
            )
        }
        self.flakes = flakes
    }

    var body: some View {
        TimelineView(.animation) { timeline in
            Canvas { context, size in
                let elapsed = timeline.date.timeIntervalSinceReferenceDate
                for flake in flakes {
                    flake.draw(in: &context, size: size, elapsed: elapsed)
                }
            }
        }
        .allowsHitTesting(false)
        .ignoresSafeArea()
        .task {
            if playsPartyHorn {
                // System party-horn sound (AudioServices one-shot — no
                // session configuration needed for a UI sound).
                AudioServicesPlaySystemSound(1104)
            }
        }
    }
}

/// One confetti flake — a pure value with its birth parameters; all
/// motion is a function of elapsed time (no mutable per-frame state,
/// which is what keeps this stutter-free).
struct ConfettiFlake {
    static let flakeCount = 140
    /// Native-adjacent festive palette (SwiftUI colors).
    static let palette: [Color] = [
        .pink, .yellow, .mint, .orange, .cyan, .indigo, .white
    ]

    let index: Int
    let startX: Double
    let startY: Double
    let horizontalDrift: Double
    let fallSpeed: Double
    let rotationSpeed: Double
    let colorIndex: Int

    /// A flake's life: ~7s of falling with fade-out over the last 1.5s.
    private static let lifeDuration: Double = 7.0
    private static let fadeDuration: Double = 1.5

    func draw(in context: inout GraphicsContext, size: CGSize, elapsed: Double) {
        let life = elapsed.truncatingRemainder(dividingBy: Self.lifeDuration)
        // Each flake starts on its own beat (staggered, not a wall).
        let flakeElapsed = life - Double(index % 12) * 0.45
        guard flakeElapsed > 0 else { return }

        let progress = flakeElapsed * fallSpeed
        let horizontalPosition = (startX + horizontalDrift * flakeElapsed) * size.width
        let verticalPosition = startY * -size.height + progress * size.height * 0.4
        guard verticalPosition < size.height else { return }

        let rotation = Angle.radians(flakeElapsed * rotationSpeed)
        let alpha = flakeElapsed > Self.lifeDuration - Self.fadeDuration
            ? max(0, (Self.lifeDuration - flakeElapsed) / Self.fadeDuration)
            : 1.0

        var flakeContext = context
        flakeContext.opacity = alpha
        flakeContext.translateBy(x: horizontalPosition, y: verticalPosition)
        flakeContext.rotate(by: rotation)
        // The flake: a small rounded rect — reads as paper confetti.
        flakeContext.fill(
            Path(roundedRect: CGRect(x: -4, y: -7, width: 8, height: 14), cornerRadius: 2),
            with: .color(Self.palette[colorIndex])
        )
    }
}

#endif

#if DEBUG && canImport(UIKit)
#Preview {
    ZStack {
        Color.black.ignoresSafeArea()
        CelebrationView()
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 48))
                .foregroundStyle(.pink)
            Text("Your AI account is live")
                .font(.title2.weight(.semibold))
        }
    }
}
#endif
