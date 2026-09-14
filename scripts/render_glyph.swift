import AppKit
import CoreText
import Foundation

guard CommandLine.arguments.count >= 3 else {
    fputs("usage: render_glyph <size> <out.png|.tiff> [menu|app]\n", stderr)
    exit(2)
}

let pixels = Int(CommandLine.arguments[1]) ?? 1024
let out = CommandLine.arguments[2]
let mode = CommandLine.arguments.count >= 4 ? CommandLine.arguments[3] : "app"
let menu = mode == "menu"
let size = CGFloat(pixels)

guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else {
    fputs("no sRGB color space\n", stderr)
    exit(1)
}

guard
    let cg = CGContext(
        data: nil,
        width: pixels,
        height: pixels,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: colorSpace,
        bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
    )
else {
    fputs("failed to create bitmap\n", stderr)
    exit(1)
}

cg.setAllowsAntialiasing(true)
cg.setShouldAntialias(true)
cg.interpolationQuality = .high
// Fully transparent canvas. Do not fill a plate / rounded square.
cg.clear(CGRect(x: 0, y: 0, width: size, height: size))

func heitiFont(ofSize fontSize: CGFloat) -> NSFont {
    NSFont(name: "STHeitiSC-Medium", size: fontSize)
        ?? NSFont(name: "HiraginoSansGB-W6", size: fontSize)
        ?? NSFont(name: "PingFangSC-Semibold", size: fontSize)
        ?? NSFont.systemFont(ofSize: fontSize, weight: .semibold)
}

func glyphPath(font: NSFont, character: String) -> CGPath? {
    var unichars = Array(character.utf16)
    var glyphs = [CGGlyph](repeating: 0, count: unichars.count)
    let ctFont = font as CTFont
    guard CTFontGetGlyphsForCharacters(ctFont, &unichars, &glyphs, unichars.count) else {
        return nil
    }
    return CTFontCreatePathForGlyph(ctFont, glyphs[0], nil)
}

/// Black 部 ink only. Scale the outline to `rect` so 16px / 18–20px stay readable.
func drawBuInk(in rect: CGRect, strokeScale: CGFloat) {
    let font = heitiFont(ofSize: max(rect.width, rect.height))
    guard let path = glyphPath(font: font, character: "部") else {
        fputs("failed to outline 部\n", stderr)
        exit(1)
    }
    let bbox = path.boundingBoxOfPath
    guard bbox.width > 1, bbox.height > 1 else {
        fputs("empty 部 bounds\n", stderr)
        exit(1)
    }
    let scale = min(rect.width / bbox.width, rect.height / bbox.height)
    let scaledW = bbox.width * scale
    let scaledH = bbox.height * scale
    let tx = rect.midX - scaledW / 2 - bbox.minX * scale
    let ty = rect.midY - scaledH / 2 - bbox.minY * scale

    cg.saveGState()
    cg.translateBy(x: tx, y: ty)
    cg.scaleBy(x: scale, y: scale)
    cg.addPath(path)
    cg.setFillColor(NSColor.black.cgColor)
    cg.setStrokeColor(NSColor.black.cgColor)
    cg.setLineJoin(.round)
    cg.setLineCap(.round)
    let stroke = (strokeScale * max(rect.width, rect.height)) / scale
    if stroke > 0 {
        cg.setLineWidth(stroke)
        cg.drawPath(using: .fillStroke)
    } else {
        cg.fillPath()
    }
    cg.restoreGState()
}

if menu {
    // Match Apple 拼: glyph only, black + alpha, no rounded square.
    // TISIconIsTemplate inverts this to a light 部 in the dark menu.
    let margin = max(0.8, size * 0.07)
    let fit = CGRect(x: margin, y: margin, width: size - margin * 2, height: size - margin * 2)
    drawBuInk(in: fit, strokeScale: 0.05)
} else {
    let inset = max(1.0, size * 0.06)
    let plate = CGRect(x: inset, y: inset, width: size - inset * 2, height: size - inset * 2)
    let corner = size * 0.18
    cg.setFillColor(NSColor(srgbRed: 0.96, green: 0.96, blue: 0.97, alpha: 1).cgColor)
    cg.addPath(CGPath(roundedRect: plate, cornerWidth: corner, cornerHeight: corner, transform: nil))
    cg.fillPath()
    let inner = plate.insetBy(dx: plate.width * 0.10, dy: plate.height * 0.10)
    drawBuInk(in: inner, strokeScale: 0.02)
}

guard let image = cg.makeImage() else {
    fputs("failed to snapshot glyph\n", stderr)
    exit(1)
}

let url = URL(fileURLWithPath: out)
let rep = NSBitmapImageRep(cgImage: image)
if out.lowercased().hasSuffix(".tiff") || out.lowercased().hasSuffix(".tif") {
    guard let tiff = rep.tiffRepresentation else {
        fputs("failed to encode tiff\n", stderr)
        exit(1)
    }
    try tiff.write(to: url)
} else {
    guard let png = rep.representation(using: .png, properties: [:]) else {
        fputs("failed to encode png\n", stderr)
        exit(1)
    }
    try png.write(to: url)
}
