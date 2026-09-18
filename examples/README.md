# Learn York

Five short lessons. Read them in order, run each one, then change something and run it again — that is how you learn to code.

## Run an example

```
york new lesson
york run .\lesson\main.yk
```

Or copy the file you want into your project's folder and run it there.

## 1. hello.yk — your first program

```york
public static void main(String[] args) {
    println("Hello, York!");
}
```

- Every program needs `public static void main(String[] args)`. It is the starting door.
- `println(...)` prints a line of text to the terminal.
- Things inside quotes are called **strings**.

## 2. vars.yk — variables and decisions

```york
int score = 10;
float hp = 95.5;
bool alive = true;
String name = "York";
```

- **Variables** hold values. Each has a type: `int` (whole numbers), `float` (numbers with a decimal point), `bool` (true or false), `String` (text).
- `=` is the assignment operator: it stores a value in a variable. `==` compares two values.
- `if (...) { } else { }` runs different code depending on a true/false condition.
- `for (int i = 0; i < 3; i++)` repeats code — here, three times. `i++` means "add one to i".

## 3. funcs.yk — your own functions

```york
bool isBig(int value) {
    if (value > 100) {
        return true;
    }
    return false;
}
```

- A **function** is a named block of code you can call again and again.
- The first word before the name is the return type — what the function sends back (`void` means it sends back nothing).
- `return` hands a value back to whoever called the function.

## 4. enums.yk — enums and switch

```york
enum Direction { North, South, East, West }
```

- An **enum** is a list of allowed values. `Direction d = Direction.South;` stores one of them.
- `switch (d) { case Direction.North: ... break; }` runs code for each possible value.
- `default:` catches any value you did not list.
- Comparing enums with `==` is safe and clear: `if (d == Direction.South) { ... }`.

## 5. structs.yk — structs, impl, and arenas

```york
struct Player {
    float pos;
    float hp;
    bool active;
}

impl Player {
    void damage(float amount) {
        this.hp -= amount;
    }
}
```

- A **struct** bundles related data into one thing — here, a Player with position, health, and an alive flag.
- An **impl** block adds behaviors to a struct. `this` refers to the exact object you are working on.
- An **Arena** is York's fast pool of memory. Push objects into it and loop over them:

```york
Arena<Player> players = new Arena(4);
players.push(Player { pos: 1.0, hp: 100.0, active: true });

for (Player p : players) {
    p.damage(10.0);
}
```

## Next steps

- `york new lesson` scaffolds a fresh project folder you can fill in yourself.
- `york run FILE` compiles and runs a York source file in one step.
- Break things on purpose. Reading errors and fixing them is the best teacher.