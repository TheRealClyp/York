---
title: Memory & Arenas
description: Zero-fragmentation memory management in York using Arena<T>.
---

York provides built-in `Arena<T>` buffers for high-performance memory allocation without manual `malloc`/`free` per item.

### Example Usage

```java
public static void main(String[] args) {
    Arena<int> nums = new Arena(16);
    nums.push(42);
    nums.set(0, 100);
    
    int val = nums.get(0);
    int last = nums.pop();
    
    println("Count: " + to_string(nums.count()));
    nums.clear();
}
```

### Supported Arena Methods
- `.push(item)`: Append item
- `.get(index)`: Fetch item at index
- `.set(index, item)`: Overwrite item
- `.pop()`: Remove and return last item
- `.last()`: Peek at last item
- `.count()`: Return item count
- `.is_empty()`: Check if empty
- `.clear()`: Reset count instantly
