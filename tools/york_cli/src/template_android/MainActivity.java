package york.mobileapp;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Toast;

/** York mobile shell — hosts the generated web app with full device access. */
public class MainActivity extends Activity {
    public static final int GRANTED = PackageManager.PERMISSION_GRANTED;

    private WebView web;
    private YorkBridge bridge;
    private ValueCallback<android.net.Uri[]> fileChooser;
    private SensorManager sm;
    private SensorEventListener accel;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        sm = (SensorManager) getSystemService(SENSOR_SERVICE);
        web = new WebView(this);
        bridge = new YorkBridge(this, web);
        web.addJavascriptInterface(bridge, "YorkNative");

        web.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest r) {
                return handle(r.getUrl());
            }
        });
        web.setWebChromeClient(new WebChromeClient() {
            @Override
            public boolean onJsAlert(WebView v, String url, String message, android.webkit.JsResult result) {
                toast(message);
                result.confirm();
                return true;
            }

            @Override
            public boolean onShowFileChooser(WebView v, ValueCallback<android.net.Uri[]> cb,
                                             FileChooserParams params) {
                fileChooser = cb;
                Intent i = params.createIntent();
                try {
                    startActivityForResult(Intent.createChooser(i, "Choose"), 9001);
                } catch (ActivityNotFoundException e) {
                    fileChooser = null;
                    return false;
                }
                return true;
            }
        });

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(true);
        s.setMediaPlaybackRequiresUserGesture(false);
        web.loadUrl("file:///android_asset/www/index.html");
        setContentView(web);
        hideSystemUi();
    }

    private boolean handle(Uri url) {
        if (url.getScheme().startsWith("http")) {
            try {
                startActivity(new Intent(Intent.ACTION_VIEW, url));
            } catch (ActivityNotFoundException e) {
                toast("no browser available");
            }
            return true;
        }
        return false;
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        if (requestCode == 9001) {
            android.net.Uri[] results = null;
            if (resultCode == RESULT_OK && data != null && data.getData() != null) {
                results = new android.net.Uri[]{ data.getData() };
                String name = (data.getData().getLastPathSegment() != null)
                        ? data.getData().getLastPathSegment() : "photo.jpg";
                if (!name.equals("york_capture")) {
                    toast("picked " + name);
                }
            }
            if (fileChooser != null) {
                fileChooser.onReceiveValue(results);
                fileChooser = null;
            }
        }
        super.onActivityResult(requestCode, resultCode, data);
    }

    /* ---------- YorkBridge access ---------- */

    SensorManager sensorManager() { return sm; }

    SensorEventListener accListener() {
        if (accel == null) {
            final WebView w = web;
            accel = new SensorEventListener() {
                @Override
                public void onSensorChanged(android.hardware.SensorEvent e) {
                    final String json = "{\"x\":" + e.values[0] + ",\"y\":" + e.values[1] + ",\"z\":" + e.values[2] + "}";
                    w.post(new Runnable() {
                        @Override public void run() {
                            w.evaluateJavascript("window.York && window.York._emit('accel', " + json + ");", null);
                        }
                    });
                }
                @Override
                public void onAccuracyChanged(android.hardware.Sensor s, int a) { }
            };
        }
        return accel;
    }

    void toast(final String message) {
        runOnUiThread(new Runnable() {
            @Override public void run() {
                Toast.makeText(MainActivity.this, message, Toast.LENGTH_LONG).show();
            }
        });
    }

    void dialog(String title, String message, boolean cancellable) {
        new AlertDialog.Builder(this)
                .setTitle(title.isEmpty() ? "York" : title)
                .setMessage(message)
                .setCancelable(cancellable)
                .setPositiveButton("OK", new android.content.DialogInterface.OnClickListener() {
                    @Override public void onClick(android.content.DialogInterface d, int w) { d.dismiss(); }
                })
                .show();
    }

    void confirmDialog(String title, String message, final YorkCallable cb) {
        new AlertDialog.Builder(this)
                .setTitle(title.isEmpty() ? "York" : title)
                .setMessage(message)
                .setPositiveButton("Yes", new android.content.DialogInterface.OnClickListener() {
                    @Override public void onClick(android.content.DialogInterface d, int w) { cb.accept(true); }
                })
                .setNegativeButton("No", new android.content.DialogInterface.OnClickListener() {
                    @Override public void onClick(android.content.DialogInterface d, int w) { cb.accept(false); }
                })
                .show();
    }

    /* ---------- permissions ---------- */

    boolean lacksLocationPermission() {
        if (Build.VERSION.SDK_INT < 23) return false;
        return checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) != GRANTED
                && checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) != GRANTED;
    }

    void requestLocation(final YorkBridge b) {
        if (Build.VERSION.SDK_INT < 23) { b.geoPermissionResult(true); return; }
        requestPermissions(new String[]{
                Manifest.permission.ACCESS_FINE_LOCATION,
                Manifest.permission.ACCESS_COARSE_LOCATION,
        }, 2000);
    }

    @Override
    public void onRequestPermissionsResult(int code, String[] perms, int[] grants) {
        if (code == 2000) {
            boolean ok = false;
            for (int g : grants) if (g == GRANTED) ok = true;
            bridge.geoPermissionResult(ok);
        }
    }

    /* ---------- misc ---------- */

    private void hideSystemUi() {
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_FULLSCREEN);
        }
    }

    @Override
    public void onBackPressed() {
        if (web != null && web.canGoBack()) web.goBack();
        else super.onBackPressed();
    }

    @Override
    protected void onDestroy() {
        if (web != null) { web.destroy(); web = null; }
        super.onDestroy();
    }
}