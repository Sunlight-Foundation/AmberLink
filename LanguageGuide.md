# 📘 Amberlink Language Guide

Welcome to Amberlink! This guide covers the syntax and features of the language. Amberlink is designed to be familiar to Java and C++ developers but with a lighter, script-like feel.

## 1. Basics

### Entry Point
Unlike Java, Amberlink does not require a `public static void main`. Code executes from top to bottom.

```java
print "Hello, World!"
```

### Comments
Use `//` for single-line comments.
```java
// This is a comment
int x = 5 // Comments can go here too
```

## 2. Variables & Types

Amberlink is statically typed, but supports type inference using `var`.

### Primitive Types
*   `int`: 32-bit signed integer.
*   `float`: 32-bit floating-point number.
*   `bool`: Boolean (`true` / `false`).
*   `char`: Single character (in single quotes).
*   `String`: Text string (compiled into the constant pool).
*   `List`: Heterogeneous dynamic collection (see [Collections](#7-collections)).
*   `void`: Used for functions that do not return a value.

### Declaration
You can declare variables using explicit types or `var`.

```java
int count = 10
String name = "Amber"
var explicit = 500
```

## 3. Control Flow

Parentheses around conditions are optional, making the code cleaner.

### If / Else
```java
if count > 5 {
    print "Count is big"
} else {
    print "Count is small"
}
```

### While Loop
```java
while count > 0 {
    print count
    count = count - 1
}
```

## 4. Functions

Functions are defined with a return type, a name, and parameters. They can be defined anywhere in the file.

```java
int add(int a, int b) {
    return a + b
}

void greet(String name) {
    print "Hello " + name
}

// Calling functions
int result = add(10, 20)
greet("Developer")
```

## 5. Arrays

Arrays are heap-allocated objects. You must specify the size when creating them.

```java
// Create an array of integers with size 5
int[] list = new int[5]

// Set values
list[0] = 42
list[1] = 100

// Access values
print list[0]
```

## 6. Classes & Objects (OOP)

Amberlink supports class-based Object-Oriented Programming.

### Defining a Class
Classes contain fields (variables) and methods (functions).

```java
class Counter {
    int value

    void increment() {
        // Use 'this' to access fields
        this.value = this.value + 1
    }
}
```

### Using Objects
Use the `new` keyword to create an instance.

```java
var c = new Counter()
c.value = 0
c.increment()
print c.value
```

## 7. Collections

`List` is a built-in dynamic collection. Create one with `new List()`, then use the `add`, `set`, `get`, and `size` methods.

```java
List items = new List()
items.add(10)
items.add("hello")
items.add(3.14)
print items.size()    // 3
print items.get(0)    // 10
items.set(1, "world")
```

The `len(_)` native also returns the item count of a `List` or `Array`, or the character count of a `String`.

## 8. Standard Library

Amberlink ships a set of built-in **native functions** implemented directly by the AVM. They are registered automatically and callable anywhere — no `import` needed.

### Conversions
| Function | Signature | Description |
|----------|-----------|-------------|
| `toString` | `String toString(value)` | Converts an `int`, `float`, `bool`, or `char` to its string form. |
| `toInt` | `int toInt(String \| int \| float \| char)` | Converts to an integer. |
| `toFloat` | `float toFloat(String \| int \| float)` | Converts to a float. |
| `abs` | `int\|float abs(number)` | Absolute value (int or float). |

### String helpers
| Function | Signature | Description |
|----------|-----------|-------------|
| `strLen` | `int strLen(String)` | Character count of a string. |
| `strCharAt` | `char strCharAt(String, int)` | Character at an index (0-based). |
| `strSubstring` | `String strSubstring(String, int, int)` | Substring from `start` of given length. |
| `strIndexOf` | `int strIndexOf(String, String)` | Index of first occurrence, or `-1`. |
| `strEquals` | `bool strEquals(String, String)` | Case-sensitive equality. |
| `strToUpper` | `String strToUpper(String)` | Uppercase copy. |
| `strToLower` | `String strToLower(String)` | Lowercase copy. |

### Math
| Function | Signature | Description |
|----------|-----------|-------------|
| `mathSqrt` | `float mathSqrt(number)` | Square root. |
| `mathPow` | `float mathPow(base, exp)` | `base` raised to `exp`. |
| `clock` | `float clock()` | Seconds since VM start (monotonic). |

### I/O and process
| Function | Signature | Description |
|----------|-----------|-------------|
| `printStr` | `void printStr(String)` | Prints a string without a trailing newline. |
| `input` | `String input()` | Reads one line from stdin. |
| `readFile` | `String readFile(String path)` | Reads a whole file (empty string on failure). |
| `writeFile` | `bool writeFile(String path, String content)` | Writes a file; returns `true` on success. |
| `exit` | `void exit(int status)` | Terminates the program with the given status. |
| `sleep` | `void sleep(int ms)` | Pauses execution for `ms` milliseconds. |

### Networking
Minimal HTTP/1.0 client (`http://` only, no TLS). Fail-soft: `""` on any failure, like `readFile`.

| Function | Signature | Description |
|----------|-----------|-------------|
| `httpGet` | `String httpGet(String url)` | Response body, or `""` on failure. |
| `httpPost` | `String httpPost(String url, String body)` | POSTs `body`, returns response body or `""`. |

### FFI (calling C)
Load shared libraries and call C functions with `int`/`String` arguments (`String` passes as `const char*`). Missing libraries give handle `0`; bad handles or symbols are runtime errors; wrong C signatures are undefined, as in C.

| Function | Signature | Description |
|----------|-----------|-------------|
| `loadLib` | `int loadLib(String path)` | Opens a library (`msvcrt.dll`, `libc.so.6`); `0` on failure. |
| `freeLib` | `bool freeLib(int handle)` | Closes a library; `false` for a bad handle. |
| `callInt` | `int callInt(int handle, String symbol, int a, int b)` | Calls `int f(int, int)`. |
| `callStr` | `int callStr(int handle, String symbol, String s)` | Calls `int f(const char*)`. |

### Collections (HashMap, LinkedList)
Dynamic collections backed by VM heap objects. Keys and values can be any built-in value.

| Function | Signature | Description |
|----------|-----------|-------------|
| `mapNew` | `HashMap mapNew()` | Creates an empty map. |
| `mapPut` | `void mapPut(HashMap, key, value)` | Inserts or replaces a key. |
| `mapGet` | `value mapGet(HashMap, key)` | Value for key, or `0` if absent. |
| `mapContainsKey` | `bool mapContainsKey(HashMap, key)` | Whether the key exists. |
| `mapRemove` | `void mapRemove(HashMap, key)` | Removes a key. |
| `mapSize` | `int mapSize(HashMap)` | Number of entries. |
| `llNew` | `LinkedList llNew()` | Creates an empty list. |
| `llAddFirst` | `void llAddFirst(LinkedList, value)` | Insert at the front. |
| `llAddLast` | `void llAddLast(LinkedList, value)` | Append at the end. |
| `llRemoveFirst` | `value llRemoveFirst(LinkedList)` | Remove and return the front. |
| `llRemoveLast` | `value llRemoveLast(LinkedList)` | Remove and return the end. |
| `llGet` | `value llGet(LinkedList, int)` | Value at an index. |
| `llSize` | `int llSize(LinkedList)` | Number of items. |
| `llEmpty` | `bool llEmpty(LinkedList)` | Whether the list is empty. |

Example:
```java
var scores = mapNew()
mapPut(scores, "alice", 90)
print mapGet(scores, "alice")        // 90
print mapContainsKey(scores, "bob")  // false

var tasks = llNew()
llAddLast(tasks, "build")
llAddLast(tasks, "test")
print llRemoveFirst(tasks)           // build
print llSize(tasks)                  // 1
```

Example:
```java
String greeting = strToUpper(strSubstring("hello world", 0, 5))
print greeting                        // HELLO
print mathSqrt(16.0)                  // 4
print mathPow(2.0, 10.0)              // 1024
print strIndexOf("amberlink", "link") // 5
```

## 9. Modules (`import`)

Amberlink supports splitting a program across multiple files with `import`. An imported file's functions, classes, and helpers become available to the importing file, and imports may chain (an imported file can import another).

### Import syntax
`import "relative/path.amb"` — the path is resolved relative to the importing file's directory, then the `stdlib/` directory, then the current directory.

```java
// main.amb
import "modlib/geometry.amb"

var p = new Point()          // Point defined in shapes.amb (via geometry.amb)
print twice(21)              // 42 (defined in shapes.amb)
print triple(7)              // 21 (defined in geometry.amb)
```

Imports are merged at compile time into a single compilation unit and produce one `.amc` output — there is no runtime linking or separate module bytecode. Dependencies are always emitted before the files that import them, and each file is included only once (import cycles are guarded).

```java
// modlib/shapes.amb — reusable module
class Point { int x; int y }
int twice(int n) { return n * 2 }
```

```java
// modlib/geometry.amb — imports another module
import "shapes.amb"
int triple(int n) { return n * 3 }
```

## 10. Archives (`.ama`)

An Amberlink Archive packages a compiled program plus its data files into one distributable file — the analog of a Java JAR for the current single-unit model.

### Building an archive
```sh
ambc examples/archive_test.amb --archive app.ama --resource welcome=examples/resources/welcome.txt
```
`--resource name=path` can be repeated to bundle multiple files. The compiled program is stored under the reserved entry `main`; every `--resource` becomes a named entry.

### Running an archive
```sh
avm app.ama
```
`avm` detects the `.ama` extension, extracts `main`, and runs it. Resources stay inside the archive — no extraction to disk.

### Reading resources at runtime
| Function | Signature | Description |
|----------|-----------|-------------|
| `readResource` | `String readResource(String)` | Contents of a bundled resource, or `""` if absent. |
| `hasResource` | `bool hasResource(String)` | Whether a bundled resource exists. |
| `resourceNames` | `String resourceNames()` | All bundled resource names, newline-separated. |

```java
print hasResource("welcome")   // true
print readResource("welcome")  // file contents
```

## 11. Threads (`spawn`/`join`)

OS threads under a global interpreter lock: one thread executes at a time, and
threads run concurrently through blocking calls (`sleep`, `input`, HTTP, FFI).
`spawn` evaluates its arguments now and runs the function on a new thread,
returning a handle; `join` blocks until it finishes and yields its return value.
Workers share globals and the heap; use `join` ordering to keep results
deterministic. A worker's runtime error surfaces when joined. The program waits
for all threads before exiting.

```java
int fib(int n) {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

var h1 = spawn fib(20)
var h2 = spawn fib(22)
print join(h1)   // 6765
print join(h2)   // 17711
```

| Function | Signature | Description |
|----------|-----------|-------------|
| `join` | `value join(int handle)` | Waits for the thread; yields its return value. |
