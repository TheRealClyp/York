---
title: System Commands
description: Hardware, OS, and file system inspection in York.
---

York v0.2.55 introduces a comprehensive suite of system inspection and file utility commands:

### Hardware & OS
- `os_cpu_count()`: Number of CPU cores.
- `os_total_memory()`: Physical RAM in bytes.
- `os_pid()`: Operating system Process ID.

### Environment & User
- `sys_username()`: Logged-in OS username.
- `sys_hostname()`: Machine hostname.
- `sys_time_str()`: Formatted timestamp string (`YYYY-MM-DD HH:MM:SS`).

### Advanced File Management
- `fs_file_size(path)`: Exact file size in bytes.
- `fs_delete_file(path)`: Deletes a file (`bool`).
- `fs_copy_file(src, dst)`: Copies file content (`bool`).

### String Helpers
- `str_slugify(s)`: Converts text to URL slug.
- `str_capitalize(s)`: Capitalizes first letter.
- `str_base64_encode(s)`: Encodes string data.
