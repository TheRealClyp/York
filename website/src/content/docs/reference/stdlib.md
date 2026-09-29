---
title: Standard Library
description: Built-in mathematical, string, and system utilities in York.
---

### Math & Trigonometry
- `min(a, b)`, `max(a, b)`, `clamp(v, lo, hi)`
- `abs(v)`, `sqrt(v)`, `pow(b, e)`, `floor(v)`, `ceil(v)`, `round(v)`
- `sin(v)`, `cos(v)`, `tan(v)`, `ln(v)`, `log10(v)`, `exp(v)`
- `srand(seed)`, `rand()`, `random_range(lo, hi)`

### String Utilities
- `.len()`, `.length()`, `.size()`
- `.contains(sub)`, `.startsWith(pre)`, `.endsWith(suf)`, `.count(sub)`
- `.indexOf(sub)`, `.lastIndexOf(sub)`
- `.toUpper()`, `.toLower()`, `.trim()`, `.reverse()`, `.replace(from, to)`
- `str_slugify(s)`, `str_capitalize(s)`, `str_base64_encode(s)`

### System & OS Commands
- `os_cpu_count()`, `os_total_memory()`, `os_pid()`
- `sys_username()`, `sys_hostname()`, `sys_time_str()`
- `fs_file_size(path)`, `fs_delete_file(path)`, `fs_copy_file(src, dst)`
- `env(name)`, `platform_name()`, `exec(cmd)`, `exit(code)`
- `now()`, `sleep_ms(ms)`
