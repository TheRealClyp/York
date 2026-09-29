//! York C backend — emits a self-contained C file from the HIR and
//! compiles it with the system C compiler (clang, gcc, or cl).

use std::collections::HashSet;

use york_ir::{self as hir, Program, Ty};

pub struct CGen {
    out: String,
    indent: usize,
    /// Is the current function `main` (needed to map York `return;` into `return 0;`).
    in_main: bool,
}

impl CGen {
    pub fn new() -> Self {
        CGen { out: String::new(), indent: 0, in_main: false }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn tag(&self, ty: &Ty) -> String {
        match ty {
            Ty::Struct(name) => name.clone(),
            Ty::Enum(name) => name.clone(),
            Ty::Arena(inner) => format!("Arena{}", self.tag(inner)),
            Ty::Slice(inner) => format!("Sl{}", self.tag(inner)),
            Ty::Str => "Str".into(),
            Ty::Bool => "Bool".into(),
            Ty::Char => "Char".into(),
            Ty::I8 => "I8".into(),
            Ty::I16 => "I16".into(),
            Ty::I32 => "I32".into(),
            Ty::I64 => "I64".into(),
            Ty::I128 => "I128".into(),
            Ty::U8 => "U8".into(),
            Ty::U16 => "U16".into(),
            Ty::U32 => "U32".into(),
            Ty::U64 => "U64".into(),
            Ty::U128 => "U128".into(),
            Ty::F32 => "F32".into(),
            Ty::F64 => "F64".into(),
            Ty::Void | Ty::Never => "Void".into(),
            Ty::Inferred => "X".into(),
        }
    }

    fn c_type(&self, ty: &Ty) -> String {
        match ty {
            Ty::Void | Ty::Never => "void".into(),
            Ty::Bool => "bool".into(),
            Ty::Char => "char".into(),
            Ty::Str => "const char*".into(),
            Ty::I8 => "signed char".into(),
            Ty::I16 => "short".into(),
            Ty::I32 => "int".into(),
            Ty::I64 => "long long".into(),
            Ty::I128 => "long long".into(),
            Ty::U8 => "unsigned char".into(),
            Ty::U16 => "unsigned short".into(),
            Ty::U32 => "unsigned int".into(),
            Ty::U64 => "unsigned long long".into(),
            Ty::U128 => "unsigned long long".into(),
            Ty::F32 => "float".into(),
            Ty::F64 => "double".into(),
            Ty::Struct(name) => name.clone(),
            Ty::Enum(name) => name.clone(),
            Ty::Arena(inner) => format!("Arena_{}", self.tag(inner)),
            Ty::Slice(inner) => format!("{}*", self.c_type(inner)),
            Ty::Inferred => "int".into(),
        }
    }

    fn default_init(&self, ty: &Ty) -> String {
        match ty {
            Ty::Arena(_) => "{ (void*)0, 0, 0 }".into(),
            Ty::Struct(_) => "{0}".into(),
            Ty::Enum(name) => format!("({name})0"),
            Ty::Str => "\"\"".into(),
            Ty::Bool => "false".into(),
            Ty::Char => "'\\0'".into(),
            _ => "0".into(),
        }
    }

    fn expr(&mut self, e: &hir::Expr) -> String {
        match e {
            hir::Expr::Int(v) => v.to_string(),
            hir::Expr::Float(v) => self.float_str(*v),
            hir::Expr::Str(s) => self.escape_string(s),
            hir::Expr::Char(c) => format!("'{}'", self.escape_char(*c)),
            hir::Expr::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            hir::Expr::Null => "0".into(),
            hir::Expr::Var(name) => name.clone(),
            hir::Expr::This => "(*self)".into(),
            hir::Expr::EnumRef { enum_name, variant } => format!("{}_{}", enum_name, variant),
            hir::Expr::Field { object, field } => {
                if matches!(&**object, hir::Expr::This) {
                    format!("self->{field}")
                } else {
                    format!("(({}).{})", self.expr(object), field)
                }
            }
            hir::Expr::MethodCall { receiver, resolved, args, .. } => {
                let arg_strs = args.iter().map(|a| self.expr(a)).collect::<Vec<_>>().join(", ");
                let joined = if arg_strs.is_empty() {
                    String::new()
                } else {
                    format!(", {arg_strs}")
                };
                // String methods take the receiver by value (it is `const char*`),
                // not by address — the receiver itself is already a pointer.
                if resolved.starts_with("__york_str_") {
                    format!("{resolved}({}{})", self.expr(receiver), joined)
                } else if resolved == "__york_strlen" {
                    format!("{resolved}({}{})", self.expr(receiver), joined)
                } else if matches!(&**receiver, hir::Expr::This) {
                    format!("{resolved}(self{joined})")
                } else {
                    format!("{resolved}(&({}){joined})", self.expr(receiver))
                }
            }
            hir::Expr::Call { resolved, args, .. } => {
                let arg_strs = args.iter().map(|a| self.expr(a)).collect::<Vec<_>>().join(", ");
                format!("{resolved}({arg_strs})")
            }
            hir::Expr::Print { text, newline, ty } => {
                let nl = if *newline { "\\n" } else { "" };
                match &**text {
                    hir::Expr::Str(s) => format!("printf(\"{}{}\")", self.escape_print(s), nl),
                    _ => {
                        let spec = format_spec(ty);
                        format!("printf(\"{spec}{nl}\", {})", self.expr(text))
                    }
                }
            }
            hir::Expr::Binary { op, left, right } => {
                format!("({} {} {})", self.expr(left), op, self.expr(right))
            }
            hir::Expr::Strcat { left, right } => {
                format!("__york_strcat({}, {})", self.expr(left), self.expr(right))
            }
            hir::Expr::Streq { is_eq, left, right } => {
                let cmp = if *is_eq { "== 0" } else { "!= 0" };
                format!("(strcmp({}, {}) {cmp})", self.expr(left), self.expr(right))
            }
            hir::Expr::Unary { op, operand } => format!("({op}{})", self.expr(operand)),
            hir::Expr::Incr { operand, positive, is_postfix } => {
                let sym = if *positive { "++" } else { "--" };
                if *is_postfix {
                    format!("({}{sym})", self.expr(operand))
                } else {
                    format!("({sym}{})", self.expr(operand))
                }
            }
            hir::Expr::Ternary { condition, then, else_ } => format!(
                "({} ? {} : {})",
                self.expr(condition),
                self.expr(then),
                self.expr(else_)
            ),
            hir::Expr::Index { object, index } => {
                format!("({})[({})]", self.expr(object), self.expr(index))
            }
            hir::Expr::StructLiteral { struct_name, fields } => {
                let mut parts = Vec::new();
                for (name, value) in fields {
                    parts.push(format!(".{name} = {}", self.expr(value)));
                }
                format!("({}){{{}}}", struct_name, parts.join(", "))
            }
            hir::Expr::New { struct_name, args, .. } => {
                // `new Arena(n)` handled at the VarDecl site; here emit generic slot init.
                if struct_name == "Arena" {
                    "((void)0)".to_string()
                } else {
                    let arg_strs = args.iter().map(|a| self.expr(a)).collect::<Vec<_>>().join(", ");
                    format!("({}){{{}}}", struct_name, arg_strs)
                }
            }
            hir::Expr::CompoundAssign { op, target, value } => {
                format!("({} {op} {})", self.expr(target), self.expr(value))
            }
            hir::Expr::Assign { target, value } => {
                format!("(({}) = ({}))", self.expr(target), self.expr(value))
            }
        }
    }

    fn float_str(&self, v: f64) -> String {
        if v == v.trunc() {
            format!("{:.0}.0", v)
        } else {
            let s = format!("{}", v);
            if s.contains("inf") || s.contains("nan") {
                s.replacen("inf", "INFINITY", 1).replace("nan", "NAN")
            } else {
                s
            }
        }
    }

    fn escape_print(&self, s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '%' => out.push_str("%%"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\r' => out.push_str("\\r"),
                _ => out.push(c),
            }
        }
        out
    }

    fn escape_string(&self, s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\r' => out.push_str("\\r"),
                _ => out.push(c),
            }
        }
        format!("\"{out}\"")
    }

    fn escape_char(&self, c: char) -> String {
        match c {
            '\'' => "\\'".into(),
            '\\' => "\\\\".into(),
            _ => c.to_string(),
        }
    }
}

fn format_spec(ty: &Ty) -> &'static str {
    match ty {
        Ty::Bool | Ty::Char | Ty::I8 | Ty::I16 | Ty::I32 | Ty::U8 | Ty::U16 | Ty::U32 => "%d",
        Ty::I64 | Ty::U64 | Ty::I128 | Ty::U128 => "%lld",
        Ty::F32 | Ty::F64 => "%g",
        Ty::Str => "%s",
        _ => "%lld",
    }
}

/// Generate C source for the whole program.
pub fn generate(program: &Program) -> String {
    let mut g = CGen::new();

    g.line("/* Generated by York -> C */");
    g.line("#include <stdio.h>");
    g.line("#include <stdlib.h>");
    g.line("#include <stdbool.h>");
    g.line("#include <stddef.h>");
    g.line("#include <stdint.h>");
    g.line("#include <math.h>");
    g.line("#include <string.h>");
    g.line("#include <ctype.h>");
    g.line("#include <time.h>");
    g.line("#ifdef _WIN32");
    g.line("#ifndef WIN32_LEAN_AND_MEAN");
    g.line("#define WIN32_LEAN_AND_MEAN");
    g.line("#endif");
    g.line("#include <windows.h>");
    g.line("#include <winsock2.h>");
    g.line("#include <ws2tcpip.h>");
    g.line("#else");
    g.line("#include <sys/socket.h>");
    g.line("#include <arpa/inet.h>");
    g.line("#include <netdb.h>");
    g.line("#include <unistd.h>");
    g.line("#include <pthread.h>");
    g.line("#endif");
    g.line("#ifdef _WIN32");
    g.line("static long long __york_last_cmd = 0;");
    g.line("static LRESULT CALLBACK __york_wndproc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {");
    g.line("    if (msg == WM_COMMAND) { __york_last_cmd = (long long)LOWORD(wParam); return 0; }");
    g.line("    if (msg == WM_DESTROY) { PostQuitMessage(0); return 0; }");
    g.line("    return DefWindowProcA(hwnd, msg, wParam, lParam);");
    g.line("}");
    g.line("static void __york_register_wndclass(void) {");
    g.line("    static int registered = 0;");
    g.line("    if (registered) return;");
    g.line("    WNDCLASSEXA wc = {0};");
    g.line("    wc.cbSize = sizeof(WNDCLASSEXA);");
    g.line("    wc.lpfnWndProc = __york_wndproc;");
    g.line("    wc.hInstance = GetModuleHandleA(NULL);");
    g.line("    wc.hCursor = LoadCursorA(NULL, IDC_ARROW);");
    g.line("    wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);");
    g.line("    wc.lpszClassName = \"YorkWindowClass\";");
    g.line("    RegisterClassExA(&wc);");
    g.line("    registered = 1;");
    g.line("}");
    g.line("static long long __york_window_create(const char* title, long long w, long long h) {");
    g.line("    __york_register_wndclass();");
    g.line("    int x = (GetSystemMetrics(SM_CXSCREEN) - (int)w) / 2;");
    g.line("    int y = (GetSystemMetrics(SM_CYSCREEN) - (int)h) / 2;");
    g.line("    HWND hwnd = CreateWindowExA(0, \"YorkWindowClass\", title ? title : \"York\", WS_OVERLAPPEDWINDOW | WS_VISIBLE, x, y, (int)w, (int)h, NULL, NULL, GetModuleHandleA(NULL), NULL);");
    g.line("    return (long long)hwnd;");
    g.line("}");
    g.line("static void __york_window_show(long long win) {");
    g.line("    ShowWindow((HWND)win, SW_SHOW);");
    g.line("    UpdateWindow((HWND)win);");
    g.line("}");
    g.line("static void __york_window_hide(long long win) { ShowWindow((HWND)win, SW_HIDE); }");
    g.line("static void __york_window_close(long long win) { DestroyWindow((HWND)win); }");
    g.line("static bool __york_window_is_open(long long win) {");
    g.line("    return IsWindow((HWND)win) != FALSE;");
    g.line("}");
    g.line("static void __york_window_poll_events(long long win) {");
    g.line("    MSG msg;");
    g.line("    while (PeekMessageA(&msg, NULL, 0, 0, PM_REMOVE)) {");
    g.line("        TranslateMessage(&msg);");
    g.line("        DispatchMessageA(&msg);");
    g.line("    }");
    g.line("}");
    g.line("static void __york_window_run_loop(long long win) {");
    g.line("    MSG msg;");
    g.line("    while (GetMessageA(&msg, NULL, 0, 0) > 0) {");
    g.line("        TranslateMessage(&msg);");
    g.line("        DispatchMessageA(&msg);");
    g.line("    }");
    g.line("}");
    g.line("static long long __york_window_last_command(void) {");
    g.line("    long long c = __york_last_cmd;");
    g.line("    __york_last_cmd = 0;");
    g.line("    return c;");
    g.line("}");
    g.line("static void __york_message_box(const char* title, const char* text) {");
    g.line("    MessageBoxA(NULL, text ? text : \"\", title ? title : \"York\", MB_OK);");
    g.line("}");
    g.line("static long long __york_control_button(long long win, const char* label, long long x, long long y, long long w, long long h, long long id) {");
    g.line("    HWND hwnd = CreateWindowExA(0, \"BUTTON\", label ? label : \"\", WS_TABSTOP | WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, (int)x, (int)y, (int)w, (int)h, (HWND)win, (HMENU)id, GetModuleHandleA(NULL), NULL);");
    g.line("    return (long long)hwnd;");
    g.line("}");
    g.line("static long long __york_control_label(long long win, const char* text, long long x, long long y, long long w, long long h, long long id) {");
    g.line("    HWND hwnd = CreateWindowExA(0, \"STATIC\", text ? text : \"\", WS_VISIBLE | WS_CHILD, (int)x, (int)y, (int)w, (int)h, (HWND)win, (HMENU)id, GetModuleHandleA(NULL), NULL);");
    g.line("    return (long long)hwnd;");
    g.line("}");
    g.line("static long long __york_control_textbox(long long win, const char* text, long long x, long long y, long long w, long long h, long long id) {");
    g.line("    HWND hwnd = CreateWindowExA(WS_EX_CLIENTEDGE, \"EDIT\", text ? text : \"\", WS_TABSTOP | WS_VISIBLE | WS_CHILD | ES_AUTOHSCROLL, (int)x, (int)y, (int)w, (int)h, (HWND)win, (HMENU)id, GetModuleHandleA(NULL), NULL);");
    g.line("    return (long long)hwnd;");
    g.line("}");
    g.line("static void __york_control_set_text(long long ctrl, const char* text) {");
    g.line("    SetWindowTextA((HWND)ctrl, text ? text : \"\");");
    g.line("}");
    g.line("#else");
    g.line("static long long __york_window_create(const char* t, long long w, long long h) { return 0; }");
    g.line("static void __york_window_show(long long w) {}");
    g.line("static void __york_window_hide(long long w) {}");
    g.line("static void __york_window_close(long long w) {}");
    g.line("static bool __york_window_is_open(long long w) { return false; }");
    g.line("static void __york_window_poll_events(long long w) {}");
    g.line("static void __york_window_run_loop(long long w) {}");
    g.line("static long long __york_window_last_command(void) { return 0; }");
    g.line("static void __york_message_box(const char* t, const char* txt) {}");
    g.line("static long long __york_control_button(long long w, const char* l, long long x, long long y, long long ww, long long hh, long long id) { return 0; }");
    g.line("static long long __york_control_label(long long w, const char* t, long long x, long long y, long long ww, long long hh, long long id) { return 0; }");
    g.line("static long long __york_control_textbox(long long w, const char* t, long long x, long long y, long long ww, long long hh, long long id) { return 0; }");
    g.line("static void __york_control_set_text(long long c, const char* t) {}");
    g.line("#endif");
    g.line("");
    g.line("/* York threads: spawn runs a user function by name in a new OS thread. */");
    g.line("static long long __york_thread_dispatch(const char* fn, long long arg);");
    g.line("static long long __york_thread_spawn(const char* fn, long long arg) {");
    g.line("    if (!fn) return -1;");
    g.line("    return __york_thread_dispatch(fn, arg);");
    g.line("}");
    g.line("static long long __york_thread_join(long long id) {");
    g.line("    #ifdef _WIN32");
    g.line("    if (id) { WaitForSingleObject((HANDLE)id, INFINITE); CloseHandle((HANDLE)id); }");
    g.line("    return 0;");
    g.line("    #else");
    g.line("    if (id) pthread_join((pthread_t)id, NULL);");
    g.line("    return 0;");
    g.line("    #endif");
    g.line("}");
    g.line("static long long __york_thread_self(void) {");
    g.line("    #ifdef _WIN32");
    g.line("    return (long long)GetCurrentThreadId();");
    g.line("    #else");
    g.line("    return (long long)pthread_self();");
    g.line("    #endif");
    g.line("}");
    g.line("");

    g.line("static bool __york_file(const char* path, const char* content, const char* mode) {");
    g.line("    FILE* f = fopen(path, mode);");
    g.line("    if (!f) return false;");
    g.line("    fputs(content, f);");
    g.line("    fclose(f);");
    g.line("    return true;");
    g.line("}");
    g.line("static bool __york_write_file(const char* path, const char* content) {");
    g.line("    return __york_file(path, content, \"wb\");");
    g.line("}");
    g.line("static bool __york_append_file(const char* path, const char* content) {");
    g.line("    return __york_file(path, content, \"ab\");");
    g.line("}");
    g.line("static bool __york_file_exists(const char* path) {");
    g.line("    FILE* f = fopen(path, \"rb\");");
    g.line("    if (!f) return false;");
    g.line("    fclose(f);");
    g.line("    return true;");
    g.line("}");
    g.line("static const char* __york_read_file(const char* path) {");
    g.line("    FILE* f = fopen(path, \"rb\");");
    g.line("    if (!f) return \"\";");
    g.line("    fseek(f, 0, SEEK_END);");
    g.line("    long sz = ftell(f);");
    g.line("    fseek(f, 0, SEEK_SET);");
    g.line("    if (sz < 0) { fclose(f); return \"\"; }");
    g.line("    char* buf = (char*)malloc((size_t)sz + 1);");
    g.line("    if (!buf) { fclose(f); return \"\"; }");
    g.line("    size_t r = fread(buf, 1, (size_t)sz, f);");
    g.line("    buf[r] = 0;");
    g.line("    fclose(f);");
    g.line("    return buf;");
    g.line("}");
    g.line("");
    g.line("static const char* __york_strcat(const char* a, const char* b) {");
    g.line("    size_t la = strlen(a), lb = strlen(b);");
    g.line("    char* out = (char*)malloc(la + lb + 1);");
    g.line("    memcpy(out, a, la);");
    g.line("    memcpy(out + la, b, lb);");
    g.line("    out[la + lb] = 0;");
    g.line("    return out;");
    g.line("}");
    g.line("static size_t __york_strlen(const char* s) {");
    g.line("    return s ? strlen(s) : 0;");
    g.line("}");
    g.line("static const char* __york_to_string_int(long long v) {");
    g.line("    char* buf = (char*)malloc(32);");
    g.line("    snprintf(buf, 32, \"%lld\", v);");
    g.line("    return buf;");
    g.line("}");
    g.line("static const char* __york_to_string_float(double v) {");
    g.line("    char* buf = (char*)malloc(32);");
    g.line("    snprintf(buf, 32, \"%g\", v);");
    g.line("    return buf;");
    g.line("}");
    g.line("static const char* __york_to_string_bool(bool v) {");
    g.line("    return v ? \"true\" : \"false\";");
    g.line("}");
    g.line("static long long __york_to_int(const char* s) {");
    g.line("    return s ? atoll(s) : 0;");
    g.line("}");
    g.line("static double __york_to_float(const char* s) {");
    g.line("    return s ? atof(s) : 0.0;");
    g.line("}");
    g.line("static int __york_exec(const char* cmd) {");
    g.line("    return system(cmd);");
    g.line("}");
    g.line("static void __york_exit(int code) {");
    g.line("    exit(code);");
    g.line("}");
    g.line("static long long __york_bin_pack(const char* path, const char* data) {");
    g.line("    FILE* f = fopen(path ? path : \"data.ybin\", \"wb\");");
    g.line("    if (!f) return 0;");
    g.line("    size_t len = data ? strlen(data) : 0;");
    g.line("    fwrite(&len, sizeof(size_t), 1, f);");
    g.line("    if (len > 0) fwrite(data, 1, len, f);");
    g.line("    fclose(f);");
    g.line("    return (long long)len;");
    g.line("}");
    g.line("static const char* __york_crypto_hash(const char* input) {");
    g.line("    unsigned long hash = 5381;");
    g.line("    if (input) { int c; while ((c = *input++)) hash = ((hash << 5) + hash) + c; }");
    g.line("    static char buf[32];");
    g.line("    snprintf(buf, sizeof(buf), \"%lx\", hash);");
    g.line("    return buf;");
    g.line("}");
    g.line("static long long __york_os_cpu_count(void) {");
    g.line("    #ifdef _WIN32");
    g.line("    SYSTEM_INFO si; GetSystemInfo(&si); return (long long)si.dwNumberOfProcessors;");
    g.line("    #else");
    g.line("    return (long long)sysconf(_SC_NPROCESSORS_ONLN);");
    g.line("    #endif");
    g.line("}");
    g.line("static long long __york_os_total_memory(void) {");
    g.line("    #ifdef _WIN32");
    g.line("    MEMORYSTATUSEX status; status.dwLength = sizeof(status); GlobalMemoryStatusEx(&status); return (long long)status.ullTotalPhys;");
    g.line("    #else");
    g.line("    return (long long)sysconf(_SC_PHYS_PAGES) * (long long)sysconf(_SC_PAGE_SIZE);");
    g.line("    #endif");
    g.line("}");
    g.line("static long long __york_os_pid(void) {");
    g.line("    #ifdef _WIN32");
    g.line("    return (long long)GetCurrentProcessId();");
    g.line("    #else");
    g.line("    return (long long)getpid();");
    g.line("    #endif");
    g.line("}");
    g.line("static const char* __york_sys_username(void) {");
    g.line("    #ifdef _WIN32");
    g.line("    static char buf[256]; DWORD len = 256; GetUserNameA(buf, &len); return buf;");
    g.line("    #else");
    g.line("    char* u = getenv(\"USER\"); return u ? u : \"root\";");
    g.line("    #endif");
    g.line("}");
    g.line("static const char* __york_sys_hostname(void) {");
    g.line("    static char buf[256];");
    g.line("    #ifdef _WIN32");
    g.line("    DWORD len = 256; GetComputerNameA(buf, &len);");
    g.line("    #else");
    g.line("    gethostname(buf, 256);");
    g.line("    #endif");
    g.line("    return buf;");
    g.line("}");
    g.line("static const char* __york_sys_time_str(void) {");
    g.line("    time_t now = time(NULL);");
    g.line("    struct tm* t = localtime(&now);");
    g.line("    static char buf[64];");
    g.line("    strftime(buf, sizeof(buf), \"%Y-%m-%d %H:%M:%S\", t);");
    g.line("    return buf;");
    g.line("}");
    g.line("static long long __york_fs_file_size(const char* path) {");
    g.line("    FILE* f = fopen(path, \"rb\");");
    g.line("    if (!f) return -1;");
    g.line("    fseek(f, 0, SEEK_END); long sz = ftell(f); fclose(f);");
    g.line("    return (long long)sz;");
    g.line("}");
    g.line("static bool __york_fs_delete_file(const char* path) {");
    g.line("    return remove(path) == 0;");
    g.line("}");
    g.line("static bool __york_fs_copy_file(const char* src, const char* dst) {");
    g.line("    FILE* sf = fopen(src, \"rb\"); if (!sf) return false;");
    g.line("    FILE* df = fopen(dst, \"wb\"); if (!df) { fclose(sf); return false; }");
    g.line("    char buf[4096]; size_t n;");
    g.line("    while ((n = fread(buf, 1, sizeof(buf), sf)) > 0) fwrite(buf, 1, n, df);");
    g.line("    fclose(sf); fclose(df);");
    g.line("    return true;");
    g.line("}");
    g.line("static const char* __york_str_slugify(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    size_t j = 0;");
    g.line("    for (size_t i = 0; i < n; i++) {");
    g.line("        char c = s[i];");
    g.line("        if ((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')) { out[j++] = c; }");
    g.line("        else if (c >= 'A' && c <= 'Z') { out[j++] = c + 32; }");
    g.line("        else if (c == ' ' || c == '-' || c == '_') { if (j == 0 || out[j-1] != '-') out[j++] = '-'; }");
    g.line("    }");
    g.line("    if (j > 0 && out[j-1] == '-') j--;");
    g.line("    out[j] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_capitalize(const char* s) {");
    g.line("    if (!s || !*s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    strcpy(out, s);");
    g.line("    out[0] = (char)toupper((unsigned char)out[0]);");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_base64_encode(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    static const char tbl[] = \"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/\";");
    g.line("    size_t len = strlen(s);");
    g.line("    size_t out_len = 4 * ((len + 2) / 3);");
    g.line("    char* out = (char*)malloc(out_len + 1);");
    g.line("    if (!out) return \"\";");
    g.line("    size_t i = 0, j = 0;");
    g.line("    while (i < len) {");
    g.line("        uint32_t a = (unsigned char)s[i++];");
    g.line("        uint32_t b = (i < len) ? (unsigned char)s[i++] : 0;");
    g.line("        uint32_t c = (i < len) ? (unsigned char)s[i++] : 0;");
    g.line("        uint32_t trip = (a << 16) | (b << 8) | c;");
    g.line("        out[j++] = tbl[(trip >> 18) & 0x3F];");
    g.line("        out[j++] = tbl[(trip >> 12) & 0x3F];");
    g.line("        out[j++] = (i > len + 1) ? '=' : tbl[(trip >> 6) & 0x3F];");
    g.line("        out[j++] = (i > len) ? '=' : tbl[trip & 0x3F];");
    g.line("    }");
    g.line("    out[out_len] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_base64_decode(const char* s) {");
    g.line("    if (!s || !*s) return \"\";");
    g.line("    size_t len = strlen(s);");
    g.line("    if (len % 4 != 0) return \"\";");
    g.line("    size_t pad = 0;");
    g.line("    if (len > 0 && s[len - 1] == '=') pad++;");
    g.line("    if (len > 1 && s[len - 2] == '=') pad++;");
    g.line("    size_t out_len = (len / 4) * 3 - pad;");
    g.line("    char* out = (char*)malloc(out_len + 1);");
    g.line("    if (!out) return \"\";");
    g.line("    int d[256];");
    g.line("    for (int k = 0; k < 256; k++) d[k] = -1;");
    g.line("    const char* tbl = \"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/\";");
    g.line("    for (int k = 0; tbl[k]; k++) d[(unsigned char)tbl[k]] = k;");
    g.line("    size_t j = 0;");
    g.line("    for (size_t i = 0; i < len; i += 4) {");
    g.line("        int v0 = d[(unsigned char)s[i]];");
    g.line("        int v1 = d[(unsigned char)s[i + 1]];");
    g.line("        int v2 = s[i + 2] == '=' ? 0 : d[(unsigned char)s[i + 2]];");
    g.line("        int v3 = s[i + 3] == '=' ? 0 : d[(unsigned char)s[i + 3]];");
    g.line("        if (v0 < 0 || v1 < 0) { free(out); return \"\"; }");
    g.line("        uint32_t trip = ((uint32_t)v0 << 18) | ((uint32_t)v1 << 12) | ((uint32_t)v2 << 6) | (uint32_t)v3;");
    g.line("        if (j < out_len) out[j++] = (char)((trip >> 16) & 0xFF);");
    g.line("        if (j < out_len) out[j++] = (char)((trip >> 8) & 0xFF);");
    g.line("        if (j < out_len) out[j++] = (char)(trip & 0xFF);");
    g.line("    }");
    g.line("    out[out_len] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static double __york_math_pi(void) { return 3.141592653589793; }");
    g.line("static double __york_math_e(void) { return 2.718281828459045; }");
    g.line("static double __york_math_lerp(double a, double b, double t) { return a + (b - a) * t; }");
    g.line("static double __york_degrees_to_radians(double d) { return d * 0.017453292519943295; }");
    g.line("static double __york_radians_to_degrees(double r) { return r * 57.29577951308232; }");
    g.line("static double __york_log2(double x) { return log(x) / log(2.0); }");
    g.line("static double __york_fract(double x) { double ip; return modf(x, &ip); }");
    g.line("static double __york_random_float(void) { return (double)rand() / (double)RAND_MAX; }");
    g.line("static double __york_math_sign(double x) { return (x > 0) - (x < 0); }");
    g.line("static bool __york_math_is_even(long long x) { return (x & 1) == 0; }");
    g.line("static bool __york_math_is_odd(long long x) { return (x & 1) != 0; }");
    g.line("static bool __york_math_is_prime(long long x) {");
    g.line("    if (x < 2) return false;");
    g.line("    if (x == 2 || x == 3) return true;");
    g.line("    if (x % 2 == 0 || x % 3 == 0) return false;");
    g.line("    for (long long i = 5; i * i <= x; i += 6) if (x % i == 0 || x % (i + 2) == 0) return false;");
    g.line("    return true;");
    g.line("}");
    g.line("static long long __york_math_gcd(long long a, long long b) {");
    g.line("    if (a < 0) a = -a; if (b < 0) b = -b;");
    g.line("    while (b) { long long t = a % b; a = b; b = t; }");
    g.line("    return a;");
    g.line("}");
    g.line("static long long __york_math_lcm(long long a, long long b) {");
    g.line("    if (a == 0 || b == 0) return 0;");
    g.line("    if (a < 0) a = -a; if (b < 0) b = -b;");
    g.line("    return (a / __york_math_gcd(a, b)) * b;");
    g.line("}");
    g.line("static bool __york_str_is_numeric(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    for (const char* p = s; *p; p++) { char c = *p; if (!(c >= '0' && c <= '9')) return false; }");
    g.line("    return true;");
    g.line("}");
    g.line("static bool __york_str_is_alnum(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    for (const char* p = s; *p; p++) { char c = *p; if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'))) return false; }");
    g.line("    return true;");
    g.line("}");
    g.line("static const char* __york_str_first(const char* s) {");
    g.line("    if (!s || !*s) return \"\";");
    g.line("    char* out = (char*)malloc(2);");
    g.line("    out[0] = s[0]; out[1] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_last(const char* s) {");
    g.line("    if (!s || !*s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(2);");
    g.line("    out[0] = s[n - 1]; out[1] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_trim_left(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    while (*s && isspace((unsigned char)*s)) s++;");
    g.line("    return s;");
    g.line("}");
    g.line("static const char* __york_str_trim_right(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    while (n > 0 && isspace((unsigned char)s[n - 1])) n--;");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    memcpy(out, s, n); out[n] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static bool __york_str_is_alpha(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    for (const char* p = s; *p; p++) if (!isalpha((unsigned char)*p)) return false;");
    g.line("    return true;");
    g.line("}");
    g.line("static bool __york_str_is_digit(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    for (const char* p = s; *p; p++) if (!isdigit((unsigned char)*p)) return false;");
    g.line("    return true;");
    g.line("}");
    g.line("static bool __york_str_is_lower(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    int has = 0;");
    g.line("    for (const char* p = s; *p; p++) { char c = *p; if (c >= 'A' && c <= 'Z') return false; if (c >= 'a' && c <= 'z') has = 1; }");
    g.line("    return has;");
    g.line("}");
    g.line("static bool __york_str_is_upper(const char* s) {");
    g.line("    if (!s || !*s) return false;");
    g.line("    int has = 0;");
    g.line("    for (const char* p = s; *p; p++) { char c = *p; if (c >= 'a' && c <= 'z') return false; if (c >= 'A' && c <= 'Z') has = 1; }");
    g.line("    return has;");
    g.line("}");
    g.line("static const char* __york_str_rev_words(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    if (!out) return \"\";");
    g.line("    char* tmp = (char*)malloc(n + 1);");
    g.line("    if (!tmp) { free(out); return \"\"; }");
    g.line("    strcpy(tmp, s);");
    g.line("    size_t r = 0; char* token = strtok(tmp, \" \\t\\n\");");
    g.line("    char* words[256]; size_t wc = 0;");
    g.line("    while (token && wc < 256) { words[wc++] = token; token = strtok(NULL, \" \\t\\n\"); }");
    g.line("    for (size_t i = wc; i > 0; i--) {");
    g.line("        size_t w = 0; while (words[i-1][w]) out[r++] = words[i-1][w++];");
    g.line("        if (i > 1) out[r++] = ' ';");
    g.line("    }");
    g.line("    out[r] = '\\0';");
    g.line("    free(tmp);");
    g.line("    return out;");
    g.line("}");
    g.line("static long long __york_str_levenshtein(const char* a, const char* b) {");
    g.line("    if (!a || !b) return -1;");
    g.line("    size_t m = strlen(a), n = strlen(b);");
    g.line("    if (m == 0) return (long long)n;");
    g.line("    if (n == 0) return (long long)m;");
    g.line("    long long* row = (long long*)malloc((n + 1) * sizeof(long long));");
    g.line("    if (!row) return -1;");
    g.line("    for (size_t j = 0; j <= n; j++) row[j] = (long long)j;");
    g.line("    for (size_t i = 1; i <= m; i++) {");
    g.line("        long long prev = row[0]; row[0] = (long long)i;");
    g.line("        for (size_t j = 1; j <= n; j++) {");
    g.line("            long long cur = row[j];");
    g.line("            long long cost = (a[i-1] == b[j-1]) ? 0 : 1;");
    g.line("            long long mn = row[j-1] + 1;");
    g.line("            if (cur + 1 < mn) mn = cur + 1;");
    g.line("            if (prev + cost < mn) mn = prev + cost;");
    g.line("            row[j] = mn; prev = cur;");
    g.line("        }");
    g.line("    }");
    g.line("    long long r = row[n]; free(row); return r;");
    g.line("}");
    g.line("static long long __york_str_word_count(const char* s) {");
    g.line("    if (!s) return 0;");
    g.line("    long long count = 0; int in_word = 0;");
    g.line("    for (const char* p = s; *p; p++) {");
    g.line("        if (isspace((unsigned char)*p)) { in_word = 0; }");
    g.line("        else if (!in_word) { in_word = 1; count++; }");
    g.line("    }");
    g.line("    return count;");
    g.line("}");
    g.line("static long long __york_net_listen(long long port) {");
    g.line("    #ifdef _WIN32");
    g.line("    WSADATA wsa; WSAStartup(MAKEWORD(2,2), &wsa);");
    g.line("    #endif");
    g.line("    int s = socket(AF_INET, SOCK_STREAM, 0);");
    g.line("    if (s < 0) return -1;");
    g.line("    int opt = 1; setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (char*)&opt, sizeof(opt));");
    g.line("    struct sockaddr_in addr;");
    g.line("    memset(&addr, 0, sizeof(addr));");
    g.line("    addr.sin_family = AF_INET;");
    g.line("    addr.sin_addr.s_addr = INADDR_ANY;");
    g.line("    addr.sin_port = htons((unsigned short)port);");
    g.line("    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) { return -1; }");
    g.line("    if (listen(s, 128) < 0) { return -1; }");
    g.line("    return (long long)s;");
    g.line("}");
    g.line("static long long __york_net_accept(long long server_sock) {");
    g.line("    struct sockaddr_in client_addr;");
    g.line("    int client_len = sizeof(client_addr);");
    g.line("    int cs = accept((int)server_sock, (struct sockaddr*)&client_addr, &client_len);");
    g.line("    return (long long)cs;");
    g.line("}");
    g.line("static long long __york_net_connect(const char* host, long long port) {");
    g.line("    #ifdef _WIN32");
    g.line("    WSADATA wsa; WSAStartup(MAKEWORD(2,2), &wsa);");
    g.line("    #endif");
    g.line("    int s = socket(AF_INET, SOCK_STREAM, 0);");
    g.line("    if (s < 0) return -1;");
    g.line("    struct hostent* he = gethostbyname(host ? host : \"localhost\");");
    g.line("    if (!he) return -1;");
    g.line("    struct sockaddr_in addr;");
    g.line("    memset(&addr, 0, sizeof(addr));");
    g.line("    addr.sin_family = AF_INET;");
    g.line("    memcpy(&addr.sin_addr, he->h_addr_list[0], he->h_length);");
    g.line("    addr.sin_port = htons((unsigned short)port);");
    g.line("    if (connect(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) { return -1; }");
    g.line("    return (long long)s;");
    g.line("}");
    g.line("static long long __york_net_send(long long sock, const char* data) {");
    g.line("    if (!data) return 0;");
    g.line("    return (long long)send((int)sock, data, (int)strlen(data), 0);");
    g.line("}");
    g.line("static const char* __york_net_recv(long long sock) {");
    g.line("    char* buf = (char*)malloc(4096);");
    g.line("    if (!buf) return \"\";");
    g.line("    int r = recv((int)sock, buf, 4095, 0);");
    g.line("    if (r <= 0) { free(buf); return \"\"; }");
    g.line("    buf[r] = '\\0';");
    g.line("    return buf;");
    g.line("}");
    g.line("static void __york_net_close(long long sock) {");
    g.line("    #ifdef _WIN32");
    g.line("    closesocket((int)sock);");
    g.line("    #else");
    g.line("    close((int)sock);");
    g.line("    #endif");
    g.line("}");
    g.line("static void __york_assert(bool cond, const char* msg) {");
    g.line("    if (!cond) {");
    g.line("        if (msg && *msg) {");
    g.line("            fprintf(stderr, \"Assertion failed: %s\\n\", msg);");
    g.line("        } else {");
    g.line("            fprintf(stderr, \"Assertion failed\\n\");");
    g.line("        }");
    g.line("        exit(1);");
    g.line("    }");
    g.line("}");
    g.line("static char* __york_read_line(void) {");
    g.line("    char* buf = (char*)malloc(1024);");
    g.line("    if (!buf) return (char*)\"\";");
    g.line("    if (!fgets(buf, 1024, stdin)) { free(buf); return (char*)\"\"; }");
    g.line("    size_t len = strlen(buf);");
    g.line("    while (len > 0 && (buf[len - 1] == '\\r' || buf[len - 1] == '\\n')) { buf[--len] = '\\0'; }");
    g.line("    return buf;");
    g.line("}");
    g.line("static long long __york_read_int(void) {");
    g.line("    char* l = __york_read_line();");
    g.line("    return atoll(l);");
    g.line("}");
    g.line("static const char* __york_str_upper(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    for (size_t i = 0; i < n; i++) out[i] = (char)toupper((unsigned char)s[i]);");
    g.line("    out[n] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_lower(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    for (size_t i = 0; i < n; i++) out[i] = (char)tolower((unsigned char)s[i]);");
    g.line("    out[n] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_trim(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s), a = 0, z = n;");
    g.line("    while (a < z && isspace((unsigned char)s[a])) a++;");
    g.line("    while (z > a && isspace((unsigned char)s[z - 1])) z--;");
    g.line("    char* out = (char*)malloc(z - a + 1);");
    g.line("    memcpy(out, s + a, z - a);");
    g.line("    out[z - a] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static bool __york_str_contains(const char* s, const char* sub) {");
    g.line("    return s && sub ? strstr(s, sub) != NULL : false;");
    g.line("}");
    g.line("static bool __york_str_starts(const char* s, const char* pre) {");
    g.line("    if (!s || !pre) return false;");
    g.line("    return strncmp(s, pre, strlen(pre)) == 0;");
    g.line("}");
    g.line("static bool __york_str_ends(const char* s, const char* suf) {");
    g.line("    if (!s || !suf) return false;");
    g.line("    size_t ls = strlen(s), lf = strlen(suf);");
    g.line("    return lf > ls ? false : strcmp(s + ls - lf, suf) == 0;");
    g.line("}");
    g.line("static const char* __york_str_sub(const char* s, long long start, long long end) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    long long a = start < 0 ? 0 : start; if (a > (long long)n) a = n;");
    g.line("    long long z = end < 0 ? (long long)n : end; if (z > (long long)n) z = n; if (z < a) z = a;");
    g.line("    size_t len = (size_t)(z - a);");
    g.line("    char* out = (char*)malloc(len + 1);");
    g.line("    memcpy(out, s + a, len);");
    g.line("    out[len] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static long long __york_str_index(const char* s, const char* sub) {");
    g.line("    if (!s || !sub) return -1;");
    g.line("    char* p = strstr(s, sub);");
    g.line("    return p ? (long long)(p - s) : -1;");
    g.line("}");
    g.line("static long long __york_str_last_index(const char* s, const char* sub) {");
    g.line("    if (!s || !sub) return -1;");
    g.line("    size_t ls = strlen(s), lb = strlen(sub);");
    g.line("    if (lb > ls || lb == 0) return -1;");
    g.line("    for (long long i = (long long)(ls - lb); i >= 0; i--) {");
    g.line("        if (strncmp(s + i, sub, lb) == 0) return i;");
    g.line("    }");
    g.line("    return -1;");
    g.line("}");
    g.line("static const char* __york_str_char_at(const char* s, long long i) {");
    g.line("    if (!s || i < 0 || (size_t)i >= strlen(s)) return \"\";");
    g.line("    char* out = (char*)malloc(2);");
    g.line("    out[0] = s[i]; out[1] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_repeat(const char* s, long long n) {");
    g.line("    if (!s || n <= 0) return \"\";");
    g.line("    size_t len = strlen(s);");
    g.line("    char* out = (char*)malloc(len * (size_t)n + 1);");
    g.line("    if (!out) return \"\";");
    g.line("    for (long long i = 0; i < n; i++) memcpy(out + (size_t)i * len, s, len);");
    g.line("    out[len * (size_t)n] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static bool __york_str_empty(const char* s) {");
    g.line("    return !s || s[0] == '\\0';");
    g.line("}");
    g.line("static const char* __york_str_replace(const char* s, const char* from, const char* to) {");
    g.line("    if (!s || !from || !*from) return s ? s : \"\";");
    g.line("    size_t n = strlen(s), lf = strlen(from), lt = strlen(to);");
    g.line("    size_t cnt = 0; const char* p = s;");
    g.line("    while ((p = strstr(p, from)) != NULL) { cnt++; p += lf; }");
    g.line("    if (cnt == 0) return s;");
    g.line("    size_t out_sz = n - cnt * lf + cnt * lt + 1;");
    g.line("    char* out = (char*)malloc(out_sz);");
    g.line("    if (!out) return s;");
    g.line("    char* w = out; p = s;");
    g.line("    for (size_t i = 0; i < cnt; i++) {");
    g.line("        char* hit = strstr(p, from);");
    g.line("        size_t pre = (size_t)(hit - p);");
    g.line("        memcpy(w, p, pre); w += pre;");
    g.line("        memcpy(w, to, lt); w += lt;");
    g.line("        p = hit + lf;");
    g.line("    }");
    g.line("    strcpy(w, p);");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_reverse(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    char* out = (char*)malloc(n + 1);");
    g.line("    for (size_t i = 0; i < n; i++) out[i] = s[n - 1 - i];");
    g.line("    out[n] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_trim_start(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    const char* p = s;");
    g.line("    while (*p && isspace((unsigned char)*p)) p++;");
    g.line("    return __york_str_sub(p, 0, (long long)strlen(p));");
    g.line("}");
    g.line("static const char* __york_str_trim_end(const char* s) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s);");
    g.line("    size_t z = n;");
    g.line("    while (z > 0 && isspace((unsigned char)s[z - 1])) z--;");
    g.line("    return __york_str_sub(s, 0, (long long)z);");
    g.line("}");
    g.line("static long long __york_now(void) {");
    g.line("    struct timespec ts;");
    g.line("    timespec_get(&ts, TIME_UTC);");
    g.line("    return (long long)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;");
    g.line("}");
    g.line("static const char* __york_env(const char* key) {");
    g.line("    const char* v = key ? getenv(key) : NULL;");
    g.line("    return v ? v : \"\";");
    g.line("}");
    g.line("static const char* __york_platform_name(void) {");
    g.line("    #if defined(_WIN32)");
    g.line("    return \"windows\";");
    g.line("    #elif defined(__APPLE__)");
    g.line("    return \"macos\";");
    g.line("    #elif defined(__linux__)");
    g.line("    return \"linux\";");
    g.line("    #elif defined(__FreeBSD__)");
    g.line("    return \"freebsd\";");
    g.line("    #else");
    g.line("    return \"unknown\";");
    g.line("    #endif");
    g.line("}");
    g.line("static long long __york_random_range(long long lo, long long hi) {");
    g.line("    if (hi <= lo) return lo;");
    g.line("    long long span = hi - lo;");
    g.line("    return lo + (long long)(rand() / (RAND_MAX / span + 1));");
    g.line("}");
    g.line("static long long __york_str_count(const char* s, const char* sub) {");
    g.line("    if (!s || !sub || !*sub) return 0;");
    g.line("    long long n = 0;");
    g.line("    size_t l = strlen(sub);");
    g.line("    const char* p = s;");
    g.line("    while ((p = strstr(p, sub)) != NULL) { n++; p += l; }");
    g.line("    return n;");
    g.line("}");
    g.line("static const char* __york_str_pad_left(const char* s, long long width, const char* pad) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s), pl = pad ? strlen(pad) : 0;");
    g.line("    if (pl == 0) pl = 1;");
    g.line("    if ((size_t)width <= n) return s;");
    g.line("    size_t need = (size_t)width - n;");
    g.line("    char* out = (char*)malloc((size_t)width + 1);");
    g.line("    if (!out) return s;");
    g.line("    for (size_t i = 0; i < need; i++) out[i] = pad ? pad[i % pl] : ' ';");
    g.line("    memcpy(out + need, s, n);");
    g.line("    out[(size_t)width] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static const char* __york_str_pad_right(const char* s, long long width, const char* pad) {");
    g.line("    if (!s) return \"\";");
    g.line("    size_t n = strlen(s), pl = pad ? strlen(pad) : 0;");
    g.line("    if (pl == 0) pl = 1;");
    g.line("    if ((size_t)width <= n) return s;");
    g.line("    size_t need = (size_t)width - n;");
    g.line("    char* out = (char*)malloc((size_t)width + 1);");
    g.line("    if (!out) return s;");
    g.line("    memcpy(out, s, n);");
    g.line("    for (size_t i = 0; i < need; i++) out[n + i] = pad ? pad[i % pl] : ' ';");
    g.line("    out[(size_t)width] = '\\0';");
    g.line("    return out;");
    g.line("}");
    g.line("static long long __york_math_min_i(long long a, long long b) { return a < b ? a : b; }");
    g.line("static long long __york_math_max_i(long long a, long long b) { return a > b ? a : b; }");
    g.line("static double __york_math_min_f(double a, double b) { return a < b ? a : b; }");
    g.line("static double __york_math_max_f(double a, double b) { return a > b ? a : b; }");
    g.line("static long long __york_math_clamp_i(long long x, long long lo, long long hi) { return x < lo ? lo : (x > hi ? hi : x); }");
    g.line("static double __york_math_clamp_f(double x, double lo, double hi) { return x < lo ? lo : (x > hi ? hi : x); }");
    g.line("static void __york_sleep(unsigned long long ms) {");
    g.line("    if (ms == 0) return;");
    g.line("    #ifdef _WIN32");
    g.line("    Sleep((DWORD)ms);");
    g.line("    #else");
    g.line("    struct timespec ts;");
    g.line("    ts.tv_sec = (time_t)(ms / 1000);");
    g.line("    ts.tv_nsec = (long)((ms % 1000) * 1000000ULL);");
    g.line("    nanosleep(&ts, NULL);");
    g.line("    #endif");
    g.line("}");
    g.line("");

    // Enum typedefs first (constants may be referenced anywhere).
    for item in &program.items {
        if let hir::Item::Enum(e) = item {
            let mut body = String::new();
            for (i, v) in e.variants.iter().enumerate() {
                if !body.is_empty() { body.push_str(", "); }
                body.push_str(&format!("{}_{}", e.name, v.name));
                // Unit variants (no fields) get an explicit value; tagged variants
                // can't be expressed as a plain C enum, so those aren't supported.
                if v.fields.is_empty() {
                    body.push_str(&format!(" = {i}"));
                }
            }
            g.line(&format!("typedef enum {{ {body} }} {name};", name = e.name));
            g.line("");
        }
    }

    // Forward-declare every struct name so arena typedefs can reference
    // element types as pointers before the full definitions exist.
    for item in &program.items {
        if let hir::Item::Struct(s) = item {
            g.line(&format!("typedef struct {name} {name};", name = s.name));
        }
    }
    g.line("");

    // Arena typedefs FIRST: struct fields may be `Arena<T>` values, so the
    // arena element-collection type must already be complete when a struct
    // that holds one is defined.
    for elem in arena_element_types(program) {
        let et = g.c_type(&elem);
        let arena_ty = format!("Arena_{}", element_tag(&elem));
        g.line(&format!("typedef struct {{ {et}* data; size_t len; size_t cap; }} {arena_ty};"));
    }
    g.line("");

    // Struct definitions now that arena element types and all struct names are known.
    for item in &program.items {
        if let hir::Item::Struct(s) = item {
            // Skip redeclaring: the typedef foward-declared this name already.
            g.line(&format!("struct {name} {{", name = s.name));
            for f in &s.fields {
                g.line(&format!("    {} {};", g.c_type(&f.ty), f.name));
            }
            g.line("};");
            g.line("");
        }
    }

    // Arena helper functions for every distinct arena element type.
    emit_arena_helpers(&mut g, program);

    // Forward declarations, then bodies.
    let fns: Vec<(hir::FnDef, bool)> = program
        .items
        .iter()
        .filter_map(|i| match i {
            hir::Item::Fn(f) => Some((f.clone(), f.name == "main")),
            _ => None,
        })
        .collect();

    for (f, _) in &fns {
        g.line(&format!("{};", fn_signature(f, &g)));
    }
    g.line("");

    for (f, is_main) in &fns {
        g.in_main = *is_main;
        g.emit_fn(f);
    }

    // Dispatcher for `thread_spawn("name", arg)` — spawns a real OS thread that
    // calls the matched user function (cast to a `long long(long long)` thunk).
    // Must come after all user functions are defined (referenced directly).
    g.line("/* thread_spawn dispatcher */");
    g.line("typedef struct { long long (*fn)(long long); long long arg; } __york_job;");
    g.line("#ifdef _WIN32");
    g.line("static DWORD WINAPI __york_thread_runner(LPVOID p) {");
    g.line("    __york_job* job = (__york_job*)p;");
    g.line("    job->fn(job->arg);");
    g.line("    free(job);");
    g.line("    return 0;");
    g.line("}");
    g.line("#else");
    g.line("static void* __york_thread_runner(void* p) {");
    g.line("    __york_job* job = (__york_job*)p;");
    g.line("    job->fn(job->arg);");
    g.line("    free(job);");
    g.line("    return NULL;");
    g.line("}");
    g.line("#endif");
    g.line("static long long __york_thread_launch(long long (*fn)(long long), long long a) {");
    g.line("    __york_job* j = (__york_job*)malloc(sizeof(__york_job));");
    g.line("    if (!j) return -1;");
    g.line("    j->fn = fn; j->arg = a;");
    g.line("    #ifdef _WIN32");
    g.line("    return (long long)CreateThread(NULL, 0, __york_thread_runner, j, 0, NULL);");
    g.line("    #else");
    g.line("    pthread_t t;");
    g.line("    if (pthread_create(&t, NULL, __york_thread_runner, j) != 0) { free(j); return -1; }");
    g.line("    return (long long)t;");
    g.line("    #endif");
    g.line("}");
    g.line("static long long __york_thread_dispatch(const char* fn, long long a) {");
    let mut dispatched = false;
    for (f, is_main) in &fns {
        if *is_main || f.name == "main" {
            continue;
        }
        // Only functions with a leading numeric param (or none) can be spawned.
        let spawnable = match f.params.first() {
            None => true,
            Some((.., ty)) => {
                matches!(
                    ty,
                    hir::Ty::I8
                        | hir::Ty::I16
                        | hir::Ty::I32
                        | hir::Ty::I64
                        | hir::Ty::I128
                        | hir::Ty::U8
                        | hir::Ty::U16
                        | hir::Ty::U32
                        | hir::Ty::U64
                        | hir::Ty::U128
                        | hir::Ty::F32
                        | hir::Ty::F64
                )
            }
        };
        if !spawnable {
            continue;
        }
        dispatched = true;
        g.line(&format!(
            "    if (strcmp(fn, \"{}\") == 0) return __york_thread_launch((long long (*)(long long)){}, a);",
            f.name,
            f.name
        ));
    }
    if !dispatched {
        g.line("    (void)fn; (void)a;");
    }
    g.line("    return -1;");
    g.line("}");

    g.out
}

fn fn_signature(f: &hir::FnDef, g: &CGen) -> String {
    let ret = if f.name == "main" {
        "int".to_string()
    } else {
        g.c_type(&f.return_ty)
    };
    if f.name == "main" {
        return format!("{ret} main(void)");
    }
    let params = f
        .params
        .iter()
        .map(|(name, ty)| {
            if name == "self" {
                format!("{}* self", g.c_type(ty))
            } else {
                format!("{} {}", g.c_type(ty), name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{ret} {}({params})", f.name)
}

impl CGen {
    fn emit_fn(&mut self, f: &hir::FnDef) {
        let sig = fn_signature(f, self);
        let is_main = f.name == "main";
        if f.body.is_empty() {
            let body = if is_main { "\n    return 0;" } else { "" };
            self.line(&format!("{sig} {{{body}\n}}"));
            self.line("");
            return;
        }
        self.line(&format!("{sig} {{"));
        self.indent += 1;
        for stmt in &f.body {
            self.emit_stmt(stmt);
        }
        if is_main {
            // main must return an int; ensure a trailing return.
            self.line("return 0;");
        }
        self.indent -= 1;
        self.line("}");
        self.line("");
    }

    fn emit_stmt(&mut self, s: &hir::Stmt) {
        match s {
            hir::Stmt::VarDecl { name, ty, init, is_const } => {
                let c_type = self.c_type(ty);
                let kw = if *is_const { "const " } else { "" };
                // Arena var decl with `new Arena(n)` initializer.
                if let Some(hir::Expr::New { struct_name, args, .. }) = init {
                    if struct_name == "Arena" {
                        let cap = args.first().map(|a| self.expr(a)).unwrap_or_else(|| "16".to_string());
                        self.line(&format!("{kw}{c_type} {name};"));
                        self.line(&format!("{c_type}_init(&{name}, {cap});"));
                        return;
                    }
                }
                match init {
                    Some(init) => {
                        let rhs = self.expr(init);
                        if rhs == "((void)0)" && matches!(self.c_type(ty).as_str(), s if s.starts_with("Arena_")) {
                            return;
                        }
                        self.line(&format!("{kw}{c_type} {name} = {rhs};"));
                    }
                    None if matches!(ty, Ty::Struct(_)) => {
                        self.line(&format!("{kw}{c_type} {name} = {{0}};"));
                    }
                    None if matches!(ty, Ty::Arena(_)) => {
                        self.line(&format!("{kw}{c_type} {name};"));
                    }
                    None => {
                        self.line(&format!("{kw}{c_type} {name} = {};", self.default_init(ty)));
                    }
                }
            }
            hir::Stmt::Expr(e) => {
                let rendered = self.expr(e);
                if rendered == "((void)0)" {
                    return;
                }
                self.line(&format!("{rendered};"));
            }
            hir::Stmt::Assign { target, value } => {
                let t = self.expr(target);
                let v = self.expr(value);
                self.line(&format!("{t} = {v};"));
            }
            hir::Stmt::Return(v) => {
                if f_ctx_main(self) {
                    self.line("return 0;");
                } else {
                    match v {
                        Some(v) => {
                            let s = self.expr(v);
                            self.line(&format!("return {s};"));
                        }
                        None => self.line("return;"),
                    }
                }
            }
            hir::Stmt::If { condition, then, else_ } => self.emit_if(condition, then, else_),
            hir::Stmt::While { condition, body } => {
                let cond = self.expr(condition);
                self.line(&format!("while ({cond}) {{"));
                self.indent += 1;
                for s in body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
            hir::Stmt::For { init, condition, update, body } => {
                let init_str = init
                    .iter()
                    .map(|s| self.for_stmt(s))
                    .collect::<Vec<_>>()
                    .join(" ");
                let cond = condition.as_ref().map(|c| self.expr(c)).unwrap_or_else(|| "1".to_string());
                let upd = update.as_ref().map(|u| self.expr(u)).unwrap_or_default();
                self.line(&format!("for ({init_str} {cond}; {upd}) {{"));
                self.indent += 1;
                for s in body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
            hir::Stmt::ForEach { var, elem_ty, iterable, body } => {
                // Snapshot the iterable once so its expression is evaluated only
                // once (avoids double evaluation when it has side effects).
                let tag = self.tag(elem_ty);
                let arena_ty = format!("Arena_{tag}");
                let elem_t = self.c_type(elem_ty);
                let it = self.expr(iterable);
                self.line(&format!("{{ {arena_ty} __it = {it};"));
                self.indent += 1;
                self.line(&format!("size_t __n = {arena_ty}_count(&__it);"));
                self.line(&format!("for (size_t __i = 0; __i < __n; ++__i) {{"));
                self.indent += 1;
                self.line(&format!("{elem_t} {var} = {arena_ty}_get(&__it, __i);"));
                for s in body {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
                self.indent -= 1;
                self.line("}");
            }
            hir::Stmt::Break => self.line("break;"),
            hir::Stmt::Continue => self.line("continue;"),
            hir::Stmt::Switch { scrutinee, arms } => self.emit_switch(scrutinee, arms),
            hir::Stmt::Block(b) => {
                self.line("{");
                self.indent += 1;
                for s in b {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
        }
    }

    fn emit_switch(&mut self, scrutinee: &hir::Expr, arms: &[hir::SwitchArm]) {
        let scrut = self.expr(scrutinee);
        self.line(&format!("switch (({scrut})) {{"));
        self.indent += 1;
        for arm in arms {
            if arm.is_default {
                self.line("default:");
            } else {
                let label = self.expr(&arm.label);
                self.line(&format!("case {}:", label));
            }
            self.indent += 1;
            for s in &arm.body {
                self.emit_stmt(s);
            }
            // C switch falls through by default; York arms are exclusive.
            self.line("break;");
            self.indent -= 1;
        }
        self.indent -= 1;
        self.line("}");
    }

    fn for_stmt(&mut self, s: &hir::Stmt) -> String {
        match s {
            hir::Stmt::VarDecl { name, ty, init, is_const } => {
                let c_type = self.c_type(ty);
                let kw = if *is_const { "const " } else { "" };
                match init {
                    Some(init) => format!("{kw}{c_type} {name} = {};", self.expr(init)),
                    None => format!("{kw}{c_type} {name} = {};", self.default_init(ty)),
                }
            }
            hir::Stmt::Expr(e) => format!("{};", self.expr(e)),
            hir::Stmt::Assign { target, value } => format!("{} = {};", self.expr(target), self.expr(value)),
            _ => String::new(),
        }
    }

    fn emit_if(&mut self, condition: &hir::Expr, then: &[hir::Stmt], else_: &[hir::Stmt]) {
        let cond = self.expr(condition);
        self.line(&format!("if ({cond}) {{"));
        self.indent += 1;
        for s in then {
            self.emit_stmt(s);
        }
        self.indent -= 1;
        if else_.is_empty() {
            self.line("}");
        } else {
            // Else-if chains.
            if let [hir::Stmt::If { condition: c, then: t, else_: e }] = else_ {
                self.line("} else");
                self.emit_if(c, t, e);
            } else {
                self.line("} else {");
                self.indent += 1;
                for s in else_ {
                    self.emit_stmt(s);
                }
                self.indent -= 1;
                self.line("}");
            }
        }
    }
}

fn f_ctx_main(g: &CGen) -> bool {
    g.in_main
}

/// Collect every distinct arena element type from the program, including
/// arena-typed locals declared inside function bodies.
fn arena_element_types(program: &Program) -> Vec<Ty> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut walk = |ty: &Ty, seen: &mut HashSet<String>, out: &mut Vec<Ty>| {
        if let Ty::Arena(inner) = ty {
            let tag = element_tag(inner);
            if seen.insert(tag) {
                out.push((**inner).clone());
            }
        }
    };
    fn walk_stmts(stmts: &[hir::Stmt], walk: &mut impl FnMut(&Ty, &mut HashSet<String>, &mut Vec<Ty>), seen: &mut HashSet<String>, out: &mut Vec<Ty>) {
        for s in stmts {
            match s {
                hir::Stmt::VarDecl { ty, .. } => walk(ty, seen, out),
                hir::Stmt::Expr(e) => {
                    if let hir::Expr::Print { text, .. } = e {
                        let _ = text;
                    }
                }
                hir::Stmt::If { then, else_, .. } => {
                    walk_stmts(then, walk, seen, out);
                    walk_stmts(else_, walk, seen, out);
                }
                hir::Stmt::While { body, .. } => walk_stmts(body, walk, seen, out),
                hir::Stmt::For { init, body, .. } => {
                    walk_stmts(init, walk, seen, out);
                    walk_stmts(body, walk, seen, out);
                }
                hir::Stmt::ForEach { elem_ty, body, .. } => {
                    walk(elem_ty, seen, out);
                    walk_stmts(body, walk, seen, out);
                }
                hir::Stmt::Block(b) => walk_stmts(b, walk, seen, out),
                hir::Stmt::Switch { arms, .. } => {
                    for arm in arms {
                        walk_stmts(&arm.body, walk, seen, out);
                    }
                }
                _ => {}
            }
        }
    }
    for item in &program.items {
        match item {
            hir::Item::Fn(f) => {
                walk(&f.return_ty, &mut seen, &mut out);
                for (_, t) in &f.params {
                    walk(t, &mut seen, &mut out);
                }
                walk_stmts(&f.body, &mut walk, &mut seen, &mut out);
            }
            hir::Item::Struct(s) => {
                for field in &s.fields {
                    walk(&field.ty, &mut seen, &mut out);
                }
            }
            _ => {}
        }
    }
    out
}

fn element_tag(ty: &Ty) -> String {
    match ty {
        Ty::Struct(name) => name.clone(),
        other => {
            let g = CGen::new();
            g.tag(other)
        }
    }
}

fn emit_arena_helpers(g: &mut CGen, program: &Program) {
    for elem in arena_element_types(program) {
        let etag = element_tag(&elem);
        let et = g.c_type(&elem);
        let arena_ty = format!("Arena_{etag}");
        g.line("/* arena helpers */");
        g.line(&format!(
            "static void {arena_ty}_init({arena_ty}* a, size_t cap) {{ a->data = ({et}*)malloc((cap?cap:16)*sizeof({et})); a->len = 0; a->cap = cap?cap:16; }}"
        ));
        g.line(&format!(
            "static void {arena_ty}_push({arena_ty}* a, {et} v) {{ if (a->len >= a->cap) {{ size_t nc = a->cap ? a->cap*2 : 16; a->data = ({et}*)realloc(a->data, nc*sizeof({et})); a->cap = nc; }} a->data[a->len++] = v; }}"
        ));
        g.line(&format!(
            "static size_t {arena_ty}_count(const {arena_ty}* a) {{ return a->len; }}"
        ));
        g.line(&format!(
            "static {et} {arena_ty}_get(const {arena_ty}* a, size_t i) {{ return a->data[i]; }}"
        ));
        g.line(&format!(
            "static void {arena_ty}_set({arena_ty}* a, size_t i, {et} v) {{ if (i < a->len) a->data[i] = v; }}"
        ));
        g.line(&format!(
            "static {et} {arena_ty}_pop({arena_ty}* a) {{ return a->len > 0 ? a->data[--a->len] : ({et}){{0}}; }}"
        ));
        g.line(&format!(
            "static {et} {arena_ty}_last(const {arena_ty}* a) {{ return a->len > 0 ? a->data[a->len - 1] : ({et}){{0}}; }}"
        ));
        g.line(&format!(
            "static void {arena_ty}_clear({arena_ty}* a) {{ a->len = 0; }}"
        ));
        g.line(&format!(
            "static bool {arena_ty}_is_empty(const {arena_ty}* a) {{ return a->len == 0; }}"
        ));
        g.line("");
    }
}

/// Compile the generated C source into an executable.
pub fn compile_c(source: &str, out_exe: &str) -> Result<(), String> {
    let c_path = format!("{out_exe}.c");
    std::fs::write(&c_path, source).map_err(|e| format!("failed to write C file: {e}"))?;

    let candidates = [("clang", "clang"), ("gcc", "gcc"), ("cc", "cc"), ("tcc", "tcc"), ("zig", "zig")];
    for (label, cmd) in &candidates {
        if has_cmd(label) {
            if *label == "zig" {
                // `zig` Compiler Driver: C compilation requires the `cc` subcommand.
                return compile_zig(&c_path, out_exe);
            }
            return compile_unix_cc(cmd, &c_path, out_exe);
        }
    }
    // MSVC via Visual Studio Build Tools (Windows).
    if let Some(vcvars) = find_vcvars() {
        return compile_msvc(&vcvars, &c_path, out_exe);
    }
    Err("no C compiler found (need clang, gcc, cc, tcc, zig, or Visual Studio's cl.exe)".into())
}

fn has_cmd(c: &str) -> bool {
    std::process::Command::new(c)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// Locate vcvars64.bat under any Visual Studio 2022 BuildTools/Community/etc.
fn find_vcvars() -> Option<String> {
    let roots = [
        r"C:\Program Files (x86)\Microsoft Visual Studio\2022",
        r"C:\Program Files\Microsoft Visual Studio\2022",
    ];
    for root in &roots {
        let rr = std::path::Path::new(root);
        if let Ok(entries) = std::fs::read_dir(rr) {
            for entry in entries.flatten() {
                let vcvars = entry.path().join("VC").join("Auxiliary").join("Build").join("vcvars64.bat");
                if vcvars.is_file() {
                    return Some(vcvars.to_string_lossy().into_owned());
                }
            }
        }
    }
    None
}

fn compile_unix_cc(cc: &str, c_path: &str, out_exe: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new(cc);
    cmd.arg("-O2").arg("-std=c11").arg("-w").arg(c_path).arg("-o").arg(out_exe).arg("-lm");
    let status = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("failed to run {cc}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("C compiler ({cc}) failed with status {status}"))
    }
}

fn compile_zig(c_path: &str, out_exe: &str) -> Result<(), String> {
    let status = std::process::Command::new("zig")
        .arg("cc")
        .arg("-O2")
        .arg("-std=c11")
        .arg("-w")
        .arg(c_path)
        .arg("-o")
        .arg(out_exe)
        .arg("-lm")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| format!("failed to run zig: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("C compiler (zig) failed with status {status}"))
    }
}

fn compile_msvc(vcvars: &str, c_path: &str, out_exe: &str) -> Result<(), String> {
    use std::process::Command;
    let dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let compile_bat = dir.join(format!("york_msvc_{}_{}.bat", std::process::id(), unique_id));
    let compile_script = format!(
        "@echo off\r\ncall \"{vcvars}\" >nul 2>&1\r\ncl /nologo /O2 /std:c11 \"{c_path}\" /Fo:\"{out_exe}.obj\" /Fe:\"{out_exe}.exe\" user32.lib gdi32.lib shell32.lib ws2_32.lib advapi32.lib\r\n"
    );
    std::fs::write(&compile_bat, compile_script)
        .map_err(|e| format!("failed to write compile bat: {e}"))?;

    let mut cmd = Command::new("cmd.exe");
    cmd.arg("/C");
    cmd.arg(compile_bat.to_string_lossy().to_string());
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run cmd: {e}"))?;
    let _ = std::fs::remove_file(&compile_bat);

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("MSVC cl.exe failed\n{stdout}\n{stderr}"))
    }
}