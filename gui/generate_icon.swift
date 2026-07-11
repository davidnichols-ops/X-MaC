#!/usr/bin/env swift
// generate_icon.swift
// Generates AppIcon.icns for X-MaC.
// Run: swift generate_icon.swift

import Cocoa
import CoreGraphics

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

func createContext(size: Int) -> CGContext {
    let cs = CGColorSpace(name: CGColorSpace.sRGB)!
    let ctx = CGContext(
        data: nil,
        width: size, height: size,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: cs,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    )!
    // Flip coordinate system so (0,0) is top-left (more natural for drawing)
    ctx.translateBy(x: 0, y: CGFloat(size))
    ctx.scaleBy(x: 1, y: -1)
    return ctx
}

func saveAsPNG(context: CGContext, url: URL) {
    guard let image = context.makeImage() else { fatalError("Cannot make image") }
    let rep = NSBitmapImageRep(cgImage: image)
    guard let data = rep.representation(using: .png, properties: [:]) else {
        fatalError("Cannot create PNG data")
    }
    try! data.write(to: url)
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

func drawIcon(size: Int) -> CGContext {
    let ctx = createContext(size: size)
    let s = CGFloat(size)
    let rect = CGRect(x: 0, y: 0, width: s, height: s)

    // --- Background: deep dark navy (#0A0D14) ---
    ctx.setFillColor(CGColor(srgbRed: 0.039, green: 0.051, blue: 0.078, alpha: 1))
    ctx.fill(rect)

    // Subtle dark radial glow at centre so the shield "floats"
    let bgGradient = CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
        colors: [
            CGColor(srgbRed: 0.08,  green: 0.12, blue: 0.22, alpha: 0.9),
            CGColor(srgbRed: 0.039, green: 0.051, blue: 0.078, alpha: 0)
        ] as CFArray,
        locations: [0, 1]
    )!
    ctx.saveGState()
    ctx.clip(to: rect)
    ctx.drawRadialGradient(
        bgGradient,
        startCenter: CGPoint(x: s * 0.5, y: s * 0.5), startRadius: 0,
        endCenter:   CGPoint(x: s * 0.5, y: s * 0.5), endRadius: s * 0.65,
        options: CGGradientDrawingOptions(rawValue: 3) // beforeStartExtends | afterEndExtends
    )
    ctx.restoreGState()

    // --- Shield path ---
    // Proportions: width=60% of canvas, height=68% of canvas, centred.
    let sw = s * 0.60          // shield width
    let sh = s * 0.68          // shield height
    let sx = (s - sw) / 2      // left edge
    let sy = (s - sh) / 2 - s * 0.02  // top edge (slight upward nudge)
    let r  = sw * 0.18         // corner radius

    func shieldPath() -> CGMutablePath {
        let path = CGMutablePath()
        // Top-left arc
        path.move(to: CGPoint(x: sx + r, y: sy))
        // Top edge → top-right arc
        path.addLine(to: CGPoint(x: sx + sw - r, y: sy))
        path.addArc(center: CGPoint(x: sx + sw - r, y: sy + r),
                    radius: r, startAngle: -.pi/2, endAngle: 0, clockwise: false)
        // Right side tapers inward toward bottom
        // Upper-right straight
        path.addLine(to: CGPoint(x: sx + sw, y: sy + sh * 0.52))
        // Curve to bottom tip
        path.addQuadCurve(
            to:           CGPoint(x: sx + sw * 0.5, y: sy + sh),
            control:      CGPoint(x: sx + sw,       y: sy + sh * 0.82)
        )
        // Curve up the left side
        path.addQuadCurve(
            to:           CGPoint(x: sx, y: sy + sh * 0.52),
            control:      CGPoint(x: sx, y: sy + sh * 0.82)
        )
        // Left straight up
        path.addLine(to: CGPoint(x: sx, y: sy + r))
        path.addArc(center: CGPoint(x: sx + r, y: sy + r),
                    radius: r, startAngle: .pi, endAngle: -.pi/2, clockwise: false)
        path.closeSubpath()
        return path
    }

    // --- Outer glow (soft halo) ---
    ctx.saveGState()
    ctx.setShadow(offset: .zero, blur: s * 0.09,
                  color: CGColor(srgbRed: 0.118, green: 0.565, blue: 1.0, alpha: 0.55))
    ctx.addPath(shieldPath())
    ctx.setFillColor(CGColor(srgbRed: 0.118, green: 0.565, blue: 1.0, alpha: 0.01))
    ctx.fillPath()
    ctx.restoreGState()

    // --- Shield fill: vertical gradient electric blue (#1E90FF → #00C8FF) ---
    let shieldClipPath = shieldPath()
    ctx.saveGState()
    ctx.addPath(shieldClipPath)
    ctx.clip()

    let shieldGradient = CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
        colors: [
            CGColor(srgbRed: 0.118, green: 0.565, blue: 1.000, alpha: 1),  // #1E90FF top
            CGColor(srgbRed: 0.000, green: 0.784, blue: 1.000, alpha: 1),  // #00C8FF bottom
        ] as CFArray,
        locations: [0, 1]
    )!
    ctx.drawLinearGradient(
        shieldGradient,
        start: CGPoint(x: s * 0.5, y: sy),
        end:   CGPoint(x: s * 0.5, y: sy + sh),
        options: []
    )

    // Inner highlight: lighter streak top-centre
    let highlightGradient = CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
        colors: [
            CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.18),
            CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.0),
        ] as CFArray,
        locations: [0, 1]
    )!
    ctx.drawRadialGradient(
        highlightGradient,
        startCenter: CGPoint(x: s * 0.5, y: sy + sh * 0.22), startRadius: 0,
        endCenter:   CGPoint(x: s * 0.5, y: sy + sh * 0.22), endRadius: sw * 0.55,
        options: []
    )
    ctx.restoreGState()

    // --- Circuit / neural-network line pattern (clipped to shield) ---
    ctx.saveGState()
    ctx.addPath(shieldClipPath)
    ctx.clip()

    let lineColor = CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.13)
    ctx.setStrokeColor(lineColor)
    ctx.setLineWidth(s * 0.008)
    ctx.setLineCap(.round)

    // Define a small set of "node" positions (relative to shield centre)
    let cx = sx + sw * 0.5
    let cy = sy + sh * 0.46

    let nodes: [CGPoint] = [
        CGPoint(x: cx,            y: cy - sh*0.28),   // 0 top centre
        CGPoint(x: cx - sw*0.30,  y: cy - sh*0.10),   // 1 upper-left
        CGPoint(x: cx + sw*0.30,  y: cy - sh*0.10),   // 2 upper-right
        CGPoint(x: cx - sw*0.22,  y: cy + sh*0.14),   // 3 lower-left
        CGPoint(x: cx + sw*0.22,  y: cy + sh*0.14),   // 4 lower-right
        CGPoint(x: cx,            y: cy + sh*0.08),    // 5 centre (behind X)
        CGPoint(x: cx - sw*0.38,  y: cy + sh*0.32),   // 6 far lower-left
        CGPoint(x: cx + sw*0.38,  y: cy + sh*0.32),   // 7 far lower-right
    ]
    // Edges
    let edges: [(Int,Int)] = [
        (0,1),(0,2),(1,2),(1,3),(2,4),(3,5),(4,5),(3,6),(4,7),(5,6),(5,7),(1,5),(2,5)
    ]
    for (a, b) in edges {
        ctx.move(to: nodes[a])
        ctx.addLine(to: nodes[b])
    }
    ctx.strokePath()

    // Small filled circles at nodes
    let dotR = s * 0.018
    ctx.setFillColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.25))
    for node in nodes {
        ctx.fillEllipse(in: CGRect(
            x: node.x - dotR, y: node.y - dotR,
            width: dotR*2, height: dotR*2
        ))
    }
    ctx.restoreGState()

    // --- Shield border (thin bright stroke) ---
    ctx.saveGState()
    ctx.addPath(shieldPath())
    ctx.setStrokeColor(CGColor(srgbRed: 0.6, green: 0.88, blue: 1.0, alpha: 0.55))
    ctx.setLineWidth(s * 0.012)
    ctx.strokePath()
    ctx.restoreGState()

    // --- Inner glow ring (second thinner inset stroke) ---
    ctx.saveGState()
    // Scale the shield down 4% to get an inset ring
    let insetScale = CGFloat(0.93)
    ctx.translateBy(x: cx, y: sy + sh * 0.5)
    ctx.scaleBy(x: insetScale, y: insetScale)
    ctx.translateBy(x: -cx, y: -(sy + sh * 0.5))
    ctx.addPath(shieldPath())
    ctx.setStrokeColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.10))
    ctx.setLineWidth(s * 0.008)
    ctx.strokePath()
    ctx.restoreGState()

    // --- "X" letterform ---
    // Draw as two thick rounded strokes crossing at shield centre
    let xCx = cx
    let xCy = cy + sh * 0.02   // slightly below visual centre of shield
    let xR  = sw * 0.155        // half-size of X

    // Shadow / glow under X
    ctx.saveGState()
    ctx.setShadow(offset: .zero, blur: s * 0.035,
                  color: CGColor(srgbRed: 0.5, green: 0.9, blue: 1.0, alpha: 0.6))
    ctx.setStrokeColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0))
    ctx.setLineWidth(s * 0.001)
    ctx.move(to: CGPoint(x: xCx - xR, y: xCy - xR))
    ctx.addLine(to: CGPoint(x: xCx + xR, y: xCy + xR))
    ctx.strokePath()
    ctx.restoreGState()

    // Actual X strokes — white with slight blue tint
    ctx.setStrokeColor(CGColor(srgbRed: 0.88, green: 0.96, blue: 1.0, alpha: 0.97))
    ctx.setLineWidth(s * 0.062)
    ctx.setLineCap(.round)

    // Stroke 1: top-left → bottom-right
    ctx.move(to: CGPoint(x: xCx - xR, y: xCy - xR))
    ctx.addLine(to: CGPoint(x: xCx + xR, y: xCy + xR))
    ctx.strokePath()

    // Stroke 2: top-right → bottom-left
    ctx.move(to: CGPoint(x: xCx + xR, y: xCy - xR))
    ctx.addLine(to: CGPoint(x: xCx - xR, y: xCy + xR))
    ctx.strokePath()

    return ctx
}

// ---------------------------------------------------------------------------
// Main: generate iconset and run iconutil
// ---------------------------------------------------------------------------

let outputDir = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let icnsPath  = outputDir.appendingPathComponent("AppIcon.icns")
let iconsetURL = FileManager.default.temporaryDirectory
    .appendingPathComponent("AppIcon.iconset")

try? FileManager.default.removeItem(at: iconsetURL)
try! FileManager.default.createDirectory(at: iconsetURL, withIntermediateDirectories: true)

// Required sizes: (filename_suffix, pixel_size)
let specs: [(String, Int)] = [
    ("icon_16x16",       16),
    ("icon_16x16@2x",    32),
    ("icon_32x32",       32),
    ("icon_32x32@2x",    64),
    ("icon_128x128",    128),
    ("icon_128x128@2x", 256),
    ("icon_256x256",    256),
    ("icon_256x256@2x", 512),
    ("icon_512x512",    512),
    ("icon_512x512@2x",1024),
]

print("Generating icon sizes...")
for (name, px) in specs {
    let ctx = drawIcon(size: px)
    let fileURL = iconsetURL.appendingPathComponent("\(name).png")
    saveAsPNG(context: ctx, url: fileURL)
    print("  \(name).png  (\(px)×\(px))")
}

print("Running iconutil...")
let proc = Process()
proc.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
proc.arguments = ["-c", "icns", "-o", icnsPath.path, iconsetURL.path]
try! proc.run()
proc.waitUntilExit()

if proc.terminationStatus == 0 {
    let attrs = try! FileManager.default.attributesOfItem(atPath: icnsPath.path)
    let size  = attrs[.size] as! Int
    print("✓ AppIcon.icns created: \(icnsPath.path) (\(size) bytes)")
} else {
    print("✗ iconutil failed (exit \(proc.terminationStatus))")
    exit(1)
}

// Clean up temp iconset
try? FileManager.default.removeItem(at: iconsetURL)
