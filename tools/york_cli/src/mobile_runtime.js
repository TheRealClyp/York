/* York mobile runtime — 100% York's own device API.
 *
 * Exposes `window.York` with the Core 12 mobile features:
 *   storage, toast, dialog (alert/confirm), vibrate, geolocation,
 *   sensors (accelerometer), battery, network, clipboard, share,
 *   openUrl, camera.
 *
 * Three transport layers, same API:
 *   1. Native shell (Android YorkBridge / iOS WKScriptMessageHandler)
 *      => async JSON-RPC over YorkNative.<call>/York._resolve.
 *   2. Signed installable web app (capacitor-style shell not required)
 *   3. Plain browser (PWA / local server) => web-platform fallbacks.
 *
 * This file is injected into every York mobile app before app.js so
 * programs can call `York.*` from the moment the UI mounts.
 */
(function () {
  "use strict";

  var uid = 0;
  var pending = {}; // id -> {resolve, reject, timer}
  var listeners = {}; // event -> [fns]   (e.g. York.on('accel', f))
  var readyHandlers = [];
  var nativeConnected = false;

  /* ---------- transport ---------- */

  function hasAndroidBridge() {
    return typeof window.YorkNative === "object" && window.YorkNative !== null;
  }
  function hasIOSBridge() {
    return !!(
      window.webkit &&
      window.webkit.messageHandlers &&
      window.webkit.messageHandlers.york
    );
  }

  function nativeSend(cmd, arg) {
    if (hasAndroidBridge()) {
      // Android bridge is always async: results arrive via York._resolve.
      window.YorkNative.call(JSON.stringify({ id: uid, cmd: cmd, arg: arg === undefined ? null : arg }));
      return { id: uid, async: true };
    }
    if (hasIOSBridge()) {
      window.webkit.messageHandlers.york.postMessage({
        id: uid,
        cmd: cmd,
        arg: arg === undefined ? null : arg,
      });
      return { id: uid, async: true };
    }
    return null;
  }

  /* Resolve a pending command. Called by the native shell. */
  window.York = window.York || {};
  window.York._resolve = function (id, err, data) {
    var entry = pending[id];
    if (!entry) return;
    delete pending[id];
    if (entry.timer) clearTimeout(entry.timer);
    if (err) entry.reject(new Error(String(err)));
    else entry.resolve(data === undefined ? null : data);
  };
  window.York._emit = function (event, data) {
    var fns = listeners[event];
    if (!fns) return;
    for (var i = 0; i < fns.length; i++) {
      try { fns[i](data); } catch (e) { /* keep going */ }
    }
  };

  function call(cmd, arg, timeoutMs) {
    return new Promise(function (resolve, reject) {
      var token = { resolve: resolve, reject: reject };
      if (nativeConnected) {
        var sent = nativeSend(cmd, arg);
        if (sent === null) { resolve(webImpl(cmd, arg)); return; }
        if (sent.async !== true) {
          // Synchronous native response object.
          if (sent instanceof Error) reject(sent);
          else resolve(sent);
          return;
        }
        token.id = sent.id;
        pending[sent.id] = token;
        token.timer = setTimeout(function () {
          if (pending[sent.id]) {
            delete pending[sent.id];
            reject(new Error("York." + cmd + " timed out"));
          }
        }, timeoutMs || 15000);
      } else {
        resolve(webImpl(cmd, arg));
      }
    });
  }

  /* ---------- web fallbacks ---------- */

  function webToast(msg) {
    var old = document.getElementById("york-toast");
    if (old && old.parentNode) old.parentNode.removeChild(old);
    var t = document.createElement("div");
    t.id = "york-toast";
    t.style.cssText =
      "position:fixed;left:50%;bottom:48px;transform:translateX(-50%);" +
      "background:rgba(20,22,28,.94);color:#e5eef6;padding:10px 18px;border-radius:999px;" +
      "font:14px/1.4 system-ui,sans-serif;box-shadow:0 6px 24px rgba(0,0,0,.45);" +
      "z-index:2147483647;max-width:82vw;border:1px solid rgba(34,211,238,.35)";
    t.textContent = msg;
    document.body.appendChild(t);
    setTimeout(function () {
      if (t.parentNode) t.parentNode.removeChild(t);
    }, 2600);
  }

  function webShare(title, text, url) {
    if (navigator.share) {
      return navigator.share({ title: title, text: text, url: url }).then(function () { return true; });
    }
    var paste = (text || title || url).toString();
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(paste).then(function () {
        webToast("Copied: " + paste.slice(0, 60));
        return false;
      });
    }
    return Promise.resolve(false);
  }

  function webCamera(source) {
    return new Promise(function (resolve, reject) {
      var input = document.createElement("input");
      input.type = "file";
      input.accept = "image/*";
      input.capture = source === "front" ? "user" : "environment";
      input.onchange = function () {
        var f = input.files && input.files[0];
        if (!f) { reject(new Error("no file")); return; }
        var r = new FileReader();
        r.onload = function () {
          var img = new Image();
          img.onload = function () {
            resolve({
              name: f.name,
              type: f.type || "image/jpeg",
              dataUrl: r.result,
              width: img.naturalWidth,
              height: img.naturalHeight,
            });
          };
          img.onerror = function () { reject(new Error("bad image")); };
          img.src = r.result;
        };
        r.onerror = function () { reject(new Error("read failed")); };
        r.readAsDataURL(f);
      };
      input.click();
    });
  }

  function webGeolocation() {
    return new Promise(function (resolve, reject) {
      if (!navigator.geolocation) { reject(new Error("geolocation unsupported")); return; }
      navigator.geolocation.getCurrentPosition(
        function (p) {
          resolve({
            latitude: p.coords.latitude,
            longitude: p.coords.longitude,
            accuracy: p.coords.accuracy,
            altitude: p.coords.altitude,
            speed: p.coords.speed,
            heading: p.coords.heading,
          });
        },
        function (e) { reject(new Error(e.message || "denied")); },
        { enableHighAccuracy: true, timeout: 12000, maximumAge: 3000 }
      );
    });
  }

  var accelId = null;
  function webAccel(onData, onErr) {
    function handler(e) {
      var a = e.accelerationIncludingGravity;
      onData({ x: a.x, y: a.y, z: a.z, timestamp: e.timeStamp });
    }
    var req = null;
    function startIos() {
      DeviceMotionEvent.requestPermission().then(function (st) {
        if (st === "granted") {
          req = handler;
          window.addEventListener("devicemotion", handler);
        } else { onErr(new Error("motion permission denied")); }
      }).catch(function () { onErr(new Error("motion permission denied")); });
    }
    if (typeof DeviceMotionEvent !== "undefined" && DeviceMotionEvent.requestPermission) {
      startIos();
    } else {
      window.addEventListener("devicemotion", handler);
      req = handler;
    }
    accelId = { remove: function () { window.removeEventListener("devicemotion", handler); } };
  }
  function webAccelStop() {
    if (accelId) { accelId.remove(); accelId = null; }
  }

  function webBattery() {
    return new Promise(function (resolve, reject) {
      if (navigator.getBattery) {
        navigator.getBattery().then(function (b) {
          resolve({ level: b.level, charging: b.charging, chargingTime: b.chargingTime, dischargingTime: b.dischargingTime });
        }, reject);
      } else {
        resolve({ level: null, charging: null, chargingTime: null, dischargingTime: null });
      }
    });
  }

  function webNetwork() {
    var conn = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
    return {
      online: navigator.onLine !== false,
      type: conn ? conn.effectiveType || conn.type || "unknown" : "unknown",
      downlink: conn ? conn.downlink : null,
    };
  }

  var lastCmd = null;
  function webImpl(cmd, arg) {
    // Heavy commands route through promises; cache to keep synchronous shape.
    switch (cmd) {
      case "storage.get":
        return { value: localStorage.getItem(String(arg.key)) };
      case "storage.set":
        localStorage.setItem(String(arg.key), String(arg.value));
        return { ok: true };
      case "storage.remove":
        localStorage.removeItem(String(arg.key));
        return { ok: true };
      case "storage.clear":
        localStorage.clear();
        return { ok: true };
      case "toast":
        webToast(String(arg && arg.message));
        return { ok: true };
      case "vibrate":
        if (navigator.vibrate) navigator.vibrate(Number(arg && arg.ms) || 200);
        return { ok: true };
      case "dialog.alert":
        window.alert(String(arg && arg.message));
        return { ok: true };
      case "dialog.confirm":
        return { ok: !!window.confirm(String(arg && arg.message)) };
      case "share":
        return webShare(arg && arg.title, arg && arg.text, arg && arg.url).then(function (done) {
          return { ok: done };
        });
      case "clipboard.write":
        if (navigator.clipboard && navigator.clipboard.writeText) {
          return navigator.clipboard.writeText(String(arg && arg.text)).then(function () { return { ok: true }; });
        }
        webToast("Copied: " + String(arg && arg.text).slice(0, 60));
        return Promise.resolve({ ok: true });
      case "clipboard.read":
        if (navigator.clipboard && navigator.clipboard.readText) {
          return navigator.clipboard.readText().then(function (t) { return { text: t }; });
        }
        return Promise.resolve({ text: "" });
      case "openUrl":
        if (arg && arg.external) { window.open(arg.url, "_blank"); }
        else { window.location.href = arg.url; }
        return { ok: true };
      case "geolocation.get":
        return webGeolocation();
      case "accelerometer.start":
        return new Promise(function (resolve, reject) {
          webAccel(function (d) { window.York._emit("accel", d); }, reject);
          resolve({ ok: true });
        });
      case "accelerometer.stop":
        webAccelStop();
        return { ok: true };
      case "battery.get":
        return webBattery();
      case "network.get":
        lastCmd = webNetwork();
        return lastCmd;
      case "camera.pick":
        return webCamera(arg && arg.source);
      default:
        return { unsupported: cmd };
    }
  }

  /* ---------- public API ---------- */

  function api() {
    return {
      ready: function (cb) {
        if (nativeConnected) cb();
        else readyHandlers.push(cb);
      },

      storage: {
        get: function (key) { return call("storage.get", { key: key }).then(function (r) { return r && r.value !== undefined ? r.value : null; }); },
        set: function (key, value) { return call("storage.set", { key: key, value: value == null ? "" : String(value) }); },
        remove: function (key) { return call("storage.remove", { key: key }); },
        clear: function () { return call("storage.clear"); },
      },

      toast: function (message) { return call("toast", { message: message }); },

      dialog: {
        alert: function (message, title) { return call("dialog.alert", { message: message, title: title || "" }); },
        confirm: function (message, title) {
          return call("dialog.confirm", { message: message, title: title || "" }).then(function (r) { return !!(r && r.ok); });
        },
      },

      vibrate: function (ms) { return call("vibrate", { ms: ms || 200 }); },

      geolocation: {
        get: function () { return call("geolocation.get"); },
      },

      sensors: {
        startAccelerometer: function (onData) {
          listeners.accel = [onData];
          return call("accelerometer.start").then(function () { return onData; });
        },
        onAccelerometer: function (onData) { listeners.accel = [onData]; },
        stopAccelerometer: function () {
          listeners.accel = [];
          return call("accelerometer.stop");
        },
      },
      on: function (event, fn) {
        (listeners[event] = listeners[event] || []).push(fn);
      },

      battery: { get: function () { return call("battery.get"); } },
      network: { get: function () { return call("network.get"); } },

      clipboard: {
        write: function (text) { return call("clipboard.write", { text: text }); },
        read: function () { return call("clipboard.read").then(function (r) { return r && r.text ? r.text : ""; }); },
      },

      share: function (title, text, url) { return call("share", { title: title, text: text, url: url }); },

      openUrl: function (url, external) { return call("openUrl", { url: url, external: !!external }); },

      camera: {
        // Camera uses the web file-chooser path: works in the native WebView
        // (onShowFileChooser -> ACTION_IMAGE_CAPTURE) and on plain browsers.
        back: function () { return webCamera("back"); },
        front: function () { return webCamera("front"); },
        pick: function () { return webCamera(); },
      },
    };
  }

  /* ---------- bootstrap ---------- */

  function detectBridge() {
    if (hasAndroidBridge()) return "android";
    if (hasIOSBridge()) return "ios";
    return null;
  }

  function boot() {
    nativeConnected = !!detectBridge();
    var Y = api();
    Y.platform = detectBridge() || "web";
    window.York = Y;

    // If a program ran before the runtime loaded (inline), patch anyway.
    var i;
    for (i = 0; i < readyHandlers.length; i++) { try { readyHandlers[i](); } catch (e) {} }
    readyHandlers = [];
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();