import Foundation
import WebKit
import CoreLocation
import CoreMotion
import UIKit
import Network

/// York mobile runtime bridge — Swift side of window.York.* (Core 12).
///
/// The same commands York.js sends on Android are implemented here with the
/// system frameworks, so one York app behaves identically on both stores.
final class YorkBridge: NSObject, WKScriptMessageHandler {
    private weak var web: WKWebView?
    private let location = CLLocationManager()
    private let motion = CMMotionManager()
    private let monitor = NWPathMonitor()

    init(web: WKWebView) {
        self.web = web
        super.init()
    }

    // MARK: WKScriptMessageHandler

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.name == "york",
              let body = message.body as? [String: Any],
              let cmd = body["cmd"] as? String else { return }
        let id = body["id"] as? Int ?? 0
        let arg = body["arg"] as? [String: Any] ?? [:]

        DispatchQueue.main.async { [weak self] in
            self?.dispatch(cmd, arg: arg, id: id)
        }
    }

    // MARK: transport

    private func resolve(_ id: Int, _ data: [String: Any]) {
        post("window.York && window.York._resolve(\(id), null, \(json(data)))")
    }

    private func reject(_ id: Int, _ message: String) {
        let esc = message.replacingOccurrences(of: "'", with: "\\'").replacingOccurrences(of: "\n", with: " ")
        post("window.York && window.York._resolve(\(id), '\(esc)', null)")
    }

    private func emit(_ event: String, _ data: [String: Any]) {
        post("window.York && window.York._emit('\(event)', \(json(data)))")
    }

    private func post(_ js: String) {
        web?.evaluateJavaScript(js, completionHandler: nil)
    }

    private func json(_ d: [String: Any]) -> String {
        guard let obj = try? JSONSerialization.data(withJSONObject: d),
              let s = String(data: obj, encoding: .utf8) else { return "null" }
        return s
    }

    private var storage: UserDefaults { .standard }

    // MARK: dispatcher

    private func dispatch(_ cmd: String, arg: [String: Any], id: Int) {
        switch cmd {
        case "storage.get":
            let key = arg["key"] as? String ?? ""
            resolve(id, ["value": storage.string(forKey: key) as Any])
        case "storage.set":
            storage.set(arg["value"] as? String ?? "", forKey: arg["key"] as? String ?? "")
            resolve(id, ["ok": true])
        case "storage.remove":
            storage.removeObject(forKey: arg["key"] as? String ?? "")
            resolve(id, ["ok": true])
        case "storage.clear":
            for k in storage.dictionaryRepresentation().keys where k.hasPrefix("york.") {
                storage.removeObject(forKey: k)
            }
            resolve(id, ["ok": true])

        case "toast":
            let msg = arg["message"] as? String ?? ""
            let alert = UIAlertController(title: nil, message: msg, preferredStyle: .alert)
            web?.window?.rootViewController?.present(alert, animated: true, completion: nil)
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.6) {
                alert.dismiss(animated: true, completion: nil)
            }
            resolve(id, ["ok": true])

        case "dialog.alert":
            let a = UIAlertController(title: arg["title"] as? String ?? "York",
                                      message: arg["message"] as? String ?? "",
                                      preferredStyle: .alert)
            a.addAction(UIAlertAction(title: "OK", style: .default))
            presenter().present(a, animated: true)
            resolve(id, ["ok": true])

        case "dialog.confirm":
            let a = UIAlertController(title: arg["title"] as? String ?? "York",
                                      message: arg["message"] as? String ?? "",
                                      preferredStyle: .alert)
            a.addAction(UIAlertAction(title: "No", style: .cancel, handler: { _ in
                self.resolve(id, ["ok": false])
            }))
            a.addAction(UIAlertAction(title: "Yes", style: .default, handler: { _ in
                self.resolve(id, ["ok": true])
            }))
            presenter().present(a, animated: true)

        case "vibrate":
            UIImpactFeedbackGenerator(style: .medium).impactOccurred()
            resolve(id, ["ok": true])

        case "geolocation.get":
            location.delegate = self
            location.requestWhenInUseAuthorization()
            location.requestLocation()
            pendingGeo = id
            locationTimeout?.invalidate()
            locationTimeout = DispatchWorkItem { [weak self] in
                guard let self, let i = self.pendingGeo else { return }
                self.pendingGeo = nil
                if let last = self.location.location {
                    self.arrive(i, last)
                } else {
                    self.reject(i, "location unavailable")
                }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 8, execute: locationTimeout!)

        case "battery.get":
            UIDevice.current.isBatteryMonitoringEnabled = true
            let level = UIDevice.current.batteryLevel
            let state = UIDevice.current.batteryState
            resolve(id, [
                "level": level >= 0 ? level : (Float(0.0) as Any),
                "charging": state == .charging || state == .full,
            ])

        case "network.get":
            if monitor.pathUpdateHandler == nil {
                monitor.pathUpdateHandler = { [weak self] path in
                    guard let self else { return }
                    let type: String
                    if path.usesInterfaceType(.wifi) { type = "wifi" }
                    else if path.usesInterfaceType(.cellular) { type = "cellular" }
                    else if path.usesInterfaceType(.wiredEthernet) { type = "ethernet" }
                    else { type = "unknown" }
                    self.emit("network", ["online": path.status == .satisfied, "type": type])
                }
                monitor.start(queue: DispatchQueue.global(qos: .background))
            }
            resolve(id, [
                "online": monitor.currentPath.status == .satisfied,
                "type": networkLabel(),
            ])

        case "clipboard.write":
            UIPasteboard.general.string = arg["text"] as? String
            resolve(id, ["ok": true])
        case "clipboard.read":
            resolve(id, ["text": UIPasteboard.general.string ?? ""])

        case "share":
            var items: [Any] = []
            if let title = arg["title"] as? String, !title.isEmpty { items.append(title) }
            if let text = arg["text"] as? String, !text.isEmpty { items.append(text) }
            if let url = arg["url"] as? String, !url.isEmpty { items.append(url) }
            let av = UIActivityViewController(activityItems: items, applicationActivities: nil)
            presenter().present(av, animated: true)
            resolve(id, ["ok": true])

        case "openUrl":
            if let url = URL(string: arg["url"] as? String ?? "") {
                UIApplication.shared.open(url)
            }
            resolve(id, ["ok": true])

        case "accelerometer.start":
            guard motion.isAccelerometerAvailable else {
                reject(id, "no accelerometer"); return
            }
            motion.accelerometerUpdateInterval = 0.2
            motion.startAccelerometerUpdates(to: .main) { [weak self] data, _ in
                guard let self, let d = data?.acceleration else { return }
                self.emit("accel", ["x": d.x, "y": d.y, "z": d.z])
            }
            resolve(id, ["ok": true])

        case "accelerometer.stop":
            motion.stopAccelerometerUpdates()
            resolve(id, ["ok": true])

        case "settings.open":
            if let url = URL(string: UIApplication.openSettingsURLString) {
                UIApplication.shared.open(url)
            }
            resolve(id, ["ok": true])

        default:
            reject(id, "unknown command: \(cmd)")
        }
    }

    // MARK: geolocation

    private var pendingGeo: Int?
    private var locationTimeout: DispatchWorkItem?

    private func arrive(_ id: Int, _ l: CLLocation) {
        let speed = l.speed >= 0 ? l.speed : (Double.nan as Any)
        let heading = l.course >= 0 ? l.course : (Double.nan as Any)
        resolve(id, [
            "latitude": l.coordinate.latitude,
            "longitude": l.coordinate.longitude,
            "accuracy": l.horizontalAccuracy,
            "altitude": l.altitude,
            "speed": speed,
            "heading": heading,
        ])
    }

    // MARK: helpers

    private func networkLabel() -> String {
        monitor.currentPath.usesInterfaceType(.wifi) ? "wifi"
            : monitor.currentPath.usesInterfaceType(.cellular) ? "cellular"
            : monitor.currentPath.usesInterfaceType(.wiredEthernet) ? "ethernet"
            : "unknown"
    }

    private func presenter() -> UIViewController {
        var vc = web?.window?.rootViewController
        while vc?.presentedViewController != nil { vc = vc?.presentedViewController }
        return vc ?? UIViewController()
    }
}

extension YorkBridge: CLLocationManagerDelegate {
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let id = pendingGeo, let loc = locations.last else { return }
        pendingGeo = nil
        locationTimeout?.cancel()
        arrive(id, loc)
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        guard let id = pendingGeo else { return }
        pendingGeo = nil
        locationTimeout?.cancel()
        reject(id, "location error: \(error.localizedDescription)")
    }
}