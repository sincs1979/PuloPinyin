import Carbon
import Foundation

guard CommandLine.arguments.count >= 2 else {
    fputs("usage: register_ime <InputMethod.app>\n", stderr)
    exit(2)
}

let path = (CommandLine.arguments[1] as NSString).standardizingPath
var isDir: ObjCBool = false
guard FileManager.default.fileExists(atPath: path, isDirectory: &isDir), isDir.boolValue else {
    fputs("not found: \(path)\n", stderr)
    exit(1)
}

let url = URL(fileURLWithPath: path) as CFURL
let registered = TISRegisterInputSource(url)
print("TISRegisterInputSource -> \(registered)")
Thread.sleep(forTimeInterval: 0.4)

func listAll() -> [TISInputSource] {
    guard let unmanaged = TISCreateInputSourceList(nil, true) else {
        return []
    }
    let array = unmanaged.takeRetainedValue()
    return (array as? [TISInputSource]) ?? []
}

func prop(_ src: TISInputSource, _ key: CFString) -> String {
    guard let ptr = TISGetInputSourceProperty(src, key) else { return "" }
    let value = Unmanaged<AnyObject>.fromOpaque(ptr).takeUnretainedValue()
    return String(describing: value)
}

let sources = listAll()
print("installed sources: \(sources.count)")

var matched: [TISInputSource] = []
for src in sources {
    let id = prop(src, kTISPropertyInputSourceID)
    let bundle = prop(src, kTISPropertyBundleID)
    let name = prop(src, kTISPropertyLocalizedName)
    let kind = prop(src, kTISPropertyInputSourceType)
    if id.localizedCaseInsensitiveContains("buluo")
        || bundle.localizedCaseInsensitiveContains("buluo")
        || name.contains("部落")
    {
        print("MATCH id=\(id) bundle=\(bundle) name=\(name) kind=\(kind)")
        matched.append(src)
    }
}

if matched.isEmpty {
    print("no buluo source yet; sample of input methods:")
    var shown = 0
    for src in sources {
        let kind = prop(src, kTISPropertyInputSourceType)
        if kind.contains("Input") || kind.contains("Method") || kind.contains("Mode") {
            print("  \(prop(src, kTISPropertyInputSourceID)) | \(prop(src, kTISPropertyLocalizedName)) | \(kind)")
            shown += 1
            if shown >= 20 { break }
        }
    }
    exit(1)
}

for src in matched {
    let status = TISEnableInputSource(src)
    print("TISEnableInputSource \(prop(src, kTISPropertyInputSourceID)) -> \(status)")
}

exit(0)
