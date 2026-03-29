import SwiftUI

/// Sigil Design System — AI-native messenger aesthetic
/// Dark-first, neon accents, monospace identity, clean hierarchy
enum SigilTheme {
    // MARK: - Colors

    /// Primary accent — electric cyan
    static let accent = Color(hex: "#00E5FF")
    /// Secondary accent — soft violet for agents
    static let agentAccent = Color(hex: "#B388FF")
    /// Success / online
    static let online = Color(hex: "#69F0AE")
    /// Warning
    static let warning = Color(hex: "#FFD740")
    /// Danger / destructive
    static let danger = Color(hex: "#FF5252")

    /// Background layers (dark mode native)
    static let bgPrimary = Color(hex: "#0A0E17")
    static let bgSecondary = Color(hex: "#111827")
    static let bgCard = Color(hex: "#1A2332")
    static let bgBubbleMine = Color(hex: "#0D47A1")
    static let bgBubbleTheirs = Color(hex: "#1E293B")

    /// Text
    static let textPrimary = Color(hex: "#F1F5F9")
    static let textSecondary = Color(hex: "#94A3B8")
    static let textMuted = Color(hex: "#64748B")

    // MARK: - Adaptive colors (light/dark)

    static var adaptiveBg: Color {
        Color(light: Color(hex: "#F8FAFC"), dark: bgPrimary)
    }
    static var adaptiveBgSecondary: Color {
        Color(light: Color(hex: "#F1F5F9"), dark: bgSecondary)
    }
    static var adaptiveCard: Color {
        Color(light: .white, dark: bgCard)
    }
    static var adaptiveText: Color {
        Color(light: Color(hex: "#0F172A"), dark: textPrimary)
    }
    static var adaptiveTextSecondary: Color {
        Color(light: Color(hex: "#475569"), dark: textSecondary)
    }
    static var adaptiveBubbleMine: Color {
        Color(light: Color(hex: "#2563EB"), dark: bgBubbleMine)
    }
    static var adaptiveBubbleTheirs: Color {
        Color(light: Color(hex: "#F1F5F9"), dark: bgBubbleTheirs)
    }

    // MARK: - Typography

    static let monoFont = Font.system(.caption, design: .monospaced)

    // MARK: - Spacing

    static let cornerRadius: CGFloat = 16
    static let bubbleRadius: CGFloat = 20
    static let cardPadding: CGFloat = 16
}

// MARK: - Color Hex Extension

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 6:
            (a, r, g, b) = (255, (int >> 16) & 0xFF, (int >> 8) & 0xFF, int & 0xFF)
        case 8:
            (a, r, g, b) = ((int >> 24) & 0xFF, (int >> 16) & 0xFF, (int >> 8) & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (255, 0, 0, 0)
        }
        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }

    init(light: Color, dark: Color) {
        #if canImport(UIKit)
        self.init(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(dark)
                : UIColor(light)
        })
        #else
        self.init(nsColor: NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .vibrantDark]) != nil
                ? NSColor(dark)
                : NSColor(light)
        })
        #endif
    }
}
