package york.mobileapp;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.location.Location;
import android.location.LocationListener;
import android.location.LocationManager;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.NetworkInfo;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.provider.Settings;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;

import org.json.JSONObject;

/** York mobile runtime bridge — every feature York.js promises, in Java. */
public class YorkBridge {
    private final MainActivity act;
    private final WebView web;
    private final SharedPreferences prefs;
    private int pendingGeoId;

    public YorkBridge(MainActivity activity, WebView webView) {
        this.act = activity;
        this.web = webView;
        this.prefs = activity.getSharedPreferences("york", Context.MODE_PRIVATE);
    }

    /* ---------- transport ---------- */

    @JavascriptInterface
    public void call(String json) {
        try {
            final JSONObject o = new JSONObject(json);
            final int id = o.optInt("id");
            final String cmd = o.getString("cmd");
            final JSONObject arg = o.optJSONObject("arg");
            final YorkBridge self = this;
            new Handler(Looper.getMainLooper()).post(new Runnable() {
                @Override public void run() {
                    try {
                        self.dispatch(cmd, arg, id);
                    } catch (Exception e) {
                        self.reject(id, e.getMessage() == null ? "error" : e.getMessage());
                    }
                }
            });
        } catch (Exception ignored) { }
    }

    private void resolve(final int id, final JSONObject data) {
        web.post(new Runnable() {
            @Override public void run() {
                web.evaluateJavascript("window.York && window.York._resolve(" + id + ", null, " +
                        (data == null ? "null" : data.toString()) + ");", null);
            }
        });
    }

    private void reject(final int id, final String message) {
        web.post(new Runnable() {
            @Override public void run() {
                String esc = message.replace("'", "\\'").replace("\n", " ");
                web.evaluateJavascript("window.York && window.York._resolve(" + id + ", '" + esc + "', null);", null);
            }
        });
    }

    private void emit(final String event, final JSONObject data) {
        web.post(new Runnable() {
            @Override public void run() {
                web.evaluateJavascript("window.York && window.York._emit('" + event + "', " + data.toString() + ");", null);
            }
        });
    }

    /* ---------- dispatcher ---------- */

    private void dispatch(String cmd, JSONObject arg, int id) throws Exception {
        if (cmd.equals("storage.get")) {
            JSONObject d = new JSONObject();
            if (!arg.has("key")) { resolve(id, d); return; }
            d.put("value", prefs.getString(str(arg, "key"), null));
            resolve(id, d);
        } else if (cmd.equals("storage.set")) {
            prefs.edit().putString(str(arg, "key"), str(arg, "value")).apply();
            resolve(id, ok());
        } else if (cmd.equals("storage.remove")) {
            prefs.edit().remove(str(arg, "key")).apply();
            resolve(id, ok());
        } else if (cmd.equals("storage.clear")) {
            prefs.edit().clear().apply();
            resolve(id, ok());

        } else if (cmd.equals("toast")) {
            act.toast(str(arg, "message"));
            resolve(id, ok());

        } else if (cmd.equals("dialog.alert")) {
            act.dialog(str(arg, "title"), str(arg, "message"), false);
            resolve(id, ok());
        } else if (cmd.equals("dialog.confirm")) {
            final int fid = id;
            act.confirmDialog(str(arg, "title"), str(arg, "message"), new YorkCallable() {
                @Override public void accept(boolean ok) {
                    JSONObject d = new JSONObject();
                    try { d.put("ok", ok); } catch (Exception ignored) { }
                    resolve(fid, d);
                }
            });

        } else if (cmd.equals("vibrate")) {
            Vibrator v = (Vibrator) act.getSystemService(Context.VIBRATOR_SERVICE);
            if (v != null && v.hasVibrator()) {
                long ms = arg != null && arg.has("ms") ? (long) arg.optDouble("ms", 200) : 200;
                if (Build.VERSION.SDK_INT >= 26) {
                    v.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE));
                } else {
                    v.vibrate(ms);
                }
            }
            resolve(id, ok());

        } else if (cmd.equals("geolocation.get")) {
            getGeo(id);

        } else if (cmd.equals("battery.get")) {
            Intent b = act.registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            int level = b == null ? -1 : b.getIntExtra("level", -1);
            int scale = b == null ? 100 : b.getIntExtra("scale", 100);
            int status = b == null ? 0 : b.getIntExtra("status", 0);
            JSONObject d = new JSONObject();
            try {
                d.put("level", level >= 0 && scale > 0 ? level * 1.0 / scale : JSONObject.NULL);
                d.put("charging", status == android.os.BatteryManager.BATTERY_STATUS_CHARGING);
            } catch (Exception ignored) { }
            resolve(id, d);

        } else if (cmd.equals("network.get")) {
            ConnectivityManager cm = (ConnectivityManager) act.getSystemService(Context.CONNECTIVITY_SERVICE);
            JSONObject d = new JSONObject();
            try { d.put("online", false); d.put("type", "none"); } catch (Exception ignored) { }
            if (cm != null) {
                if (Build.VERSION.SDK_INT >= 29) {
                    NetworkCapabilities nc = cm.getNetworkCapabilities(cm.getActiveNetwork());
                    if (nc != null) {
                        String type = nc.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ? "wifi"
                                : nc.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) ? "cellular"
                                : nc.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) ? "ethernet" : "unknown";
                        try { d.put("online", true); d.put("type", type); } catch (Exception ignored) { }
                    }
                } else {
                    NetworkInfo ni = cm.getActiveNetworkInfo();
                    if (ni != null) {
                        try { d.put("online", ni.isConnected()); d.put("type", ni.getTypeName().toLowerCase()); }
                        catch (Exception ignored) { }
                    }
                }
            }
            resolve(id, d);

        } else if (cmd.equals("clipboard.write")) {
            ClipboardManager cl = (ClipboardManager) act.getSystemService(Context.CLIPBOARD_SERVICE);
            if (cl != null) cl.setPrimaryClip(ClipData.newPlainText("york", str(arg, "text")));
            resolve(id, ok());
        } else if (cmd.equals("clipboard.read")) {
            ClipboardManager cl = (ClipboardManager) act.getSystemService(Context.CLIPBOARD_SERVICE);
            JSONObject d = new JSONObject();
            try {
                d.put("text", cl != null && cl.hasPrimaryClip()
                        ? String.valueOf(cl.getPrimaryClip().getItemAt(0).getText()) : "");
            } catch (Exception ignored) { }
            resolve(id, d);

        } else if (cmd.equals("share")) {
            String title = str(arg, "title");
            String text = str(arg, "text");
            String url = str(arg, "url");
            if (!url.isEmpty()) text = text.isEmpty() ? url : text + "\n" + url;
            Intent s = new Intent(Intent.ACTION_SEND);
            s.setType("text/plain");
            s.putExtra(Intent.EXTRA_SUBJECT, title);
            s.putExtra(Intent.EXTRA_TEXT, text);
            act.startActivity(Intent.createChooser(s, "Share via York"));
            resolve(id, ok());

        } else if (cmd.equals("openUrl")) {
            act.startActivity(new Intent(Intent.ACTION_VIEW, android.net.Uri.parse(str(arg, "url"))));
            resolve(id, ok());
        } else if (cmd.equals("settings.open")) {
            act.startActivity(new Intent(Settings.ACTION_SETTINGS));
            resolve(id, ok());

        } else if (cmd.equals("accelerometer.start")) {
            if (act.sensorManager() == null) {
                reject(id, "no sensor service"); return;
            }
            Sensor acc = act.sensorManager().getDefaultSensor(Sensor.TYPE_ACCELEROMETER);
            if (acc == null) { reject(id, "no accelerometer"); return; }
            act.sensorManager().registerListener(act.accListener(), acc, SensorManager.SENSOR_DELAY_UI);
            resolve(id, ok());
        } else if (cmd.equals("accelerometer.stop")) {
            if (act.sensorManager() != null) act.sensorManager().unregisterListener(act.accListener());
            resolve(id, ok());

        } else {
            reject(id, "unknown command: " + cmd);
        }
    }

    /* ---------- geolocation (one-shot with runtime permission) ---------- */

    private void getGeo(final int id) {
        if (act.lacksLocationPermission()) {
            pendingGeoId = id;
            act.requestLocation(this);
            return;
        }
        geoOnce(id);
    }

    /** Called from MainActivity after the runtime permission prompt. */
    public void geoPermissionResult(boolean granted) {
        if (granted) {
            geoOnce(pendingGeoId);
        } else {
            reject(pendingGeoId, "location permission denied");
        }
    }

    private void geoOnce(final int id) {
        final YorkBridge self = this;
        final LocationManager lm = (LocationManager) act.getSystemService(Context.LOCATION_SERVICE);
        Location last = null;
        if (lm != null) {
            try {
                last = lm.getLastKnownLocation(LocationManager.GPS_PROVIDER);
                if (last == null) last = lm.getLastKnownLocation(LocationManager.NETWORK_PROVIDER);
                final Location flast = last;
                lm.requestSingleUpdate(LocationManager.GPS_PROVIDER, new LocationListener() {
                    @Override public void onLocationChanged(Location l) { self.arrive(id, l); }
                    @Override public void onStatusChanged(String p, int s, android.os.Bundle b) {}
                    @Override public void onProviderEnabled(String p) {}
                    @Override public void onProviderDisabled(String p) {}
                }, Looper.getMainLooper());
            } catch (SecurityException ignored) {
                // handled below via last-known or reject
            }
        }
        if (last != null) arrive(id, last);
        else reject(id, "location unavailable");
    }

    private void arrive(int id, Location l) {
        try {
            JSONObject d = new JSONObject();
            d.put("latitude", l.getLatitude());
            d.put("longitude", l.getLongitude());
            d.put("accuracy", l.hasAccuracy() ? l.getAccuracy() : JSONObject.NULL);
            d.put("altitude", l.hasAltitude() ? l.getAltitude() : JSONObject.NULL);
            d.put("speed", l.hasSpeed() ? l.getSpeed() : JSONObject.NULL);
            d.put("heading", l.hasBearing() ? l.getBearing() : JSONObject.NULL);
            resolve(id, d);
        } catch (Exception e) {
            reject(id, "gps error");
        }
    }

    private String str(JSONObject o, String key) {
        if (o == null || !o.has(key)) return "";
        return o.optString(key, "");
    }

    private JSONObject ok() {
        try { return new JSONObject().put("ok", true); } catch (Exception e) { return new JSONObject(); }
    }
}