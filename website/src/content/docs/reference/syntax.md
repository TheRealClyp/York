---
title: Syntax & Semantics
description: Complete grammar and syntax reference for York.
---

York combines clean, modern ergonomics with bare-metal systems performance.

### Primitives & Variables
- `int`: 64-bit signed integer (`long long`)
- `float`: 64-bit IEEE double-precision float (`double`)
- `bool`: Boolean (`true` / `false`)
- `char`: Single character
- `String`: Heap-allocated null-terminated string (`const char*`)

### Control Flow
```java
if (x > 10) {
    println("Greater");
} else {
    println("Smaller");
}

while (i < 5) {
    i = i + 1;
}

for (int j = 0; j < 10; j++) {
    println(j);
}
```

### Structs & Methods
```java
struct Player {
    float hp;
    float score;
}

impl Player {
    void reset() {
        this.hp = 100.0;
        this.score = 0.0;
    }
}
```
