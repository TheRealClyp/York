---
title: Quick Start
description: Write your first York program and run it.
---

Create your first York source file (`main.yk`):

```java
public static void main(String[] args) {
    println("Hello, World from York v0.2.55!");
}
```

### Running and Building

- **Run directly**:
  ```bash
  york run main.yk
  ```
- **Compile to standalone native binary**:
  ```bash
  york build main.yk
  ```
- **Check syntax without building**:
  ```bash
  york check main.yk
  ```
