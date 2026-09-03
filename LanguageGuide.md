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

Example:
```java
String greeting = strToUpper(strSubstring("hello world", 0, 5))
print greeting                        // HELLO
print mathSqrt(16.0)                  // 4
print mathPow(2.0, 10.0)              // 1024
print strIndexOf("amberlink", "link") // 5
```