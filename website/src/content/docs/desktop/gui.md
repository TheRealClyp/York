---
title: Native Win32 GUI
description: Build native Windows desktop applications entirely in York.
---

York features a first-class native Win32 GUI framework bridging directly to Win32 `user32` and `gdi32`:

```java
public static void main(String[] args) {
    long win = window_create("York Desktop App", 800, 500);
    long btn = control_button(win, "Click Me", 50, 50, 120, 35, 101);
    long lbl = control_label(win, "Status: Ready", 50, 100, 300, 25, 102);

    window_show(win);

    while (window_is_open(win)) {
        window_poll_events(win);
        long cmd = window_last_command();
        if (cmd == 101) {
            control_set_text(lbl, "Button was clicked!");
            message_box("Info", "Hello from native York GUI!");
        }
        sleep_ms(15);
    }
    window_close(win);
}
```

### GUI API Reference
- `window_create(title, w, h)`, `window_show(win)`, `window_hide(win)`, `window_close(win)`
- `window_is_open(win)`, `window_poll_events(win)`, `window_run_loop(win)`, `window_last_command()`
- `control_button(...)`, `control_label(...)`, `control_textbox(...)`, `control_set_text(...)`
- `message_box(title, text)`
