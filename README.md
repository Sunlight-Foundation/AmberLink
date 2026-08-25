# Amberlink 🔶

A Java alternative that doesn't hate you.

Amberlink is a statically typed, multi-paradigm language built to give Java developers a cleaner home. Same familiar structure — classes, interfaces, strong typing — without the ceremony. Run on the Amber Virtual Machine or compile to a native binary. Your choice.

> **Status: Dormant — 2.0 revival in planning. Watch this repo for updates.**

---

## Why Amberlink?

Java is fine. We just think it can be better.

- No `public static void main` — code runs top to bottom
- No primitive vs object split — `int` is `int`, everywhere
- Null safety built in from the start
- Dual-backend: AVM for managed execution, native for raw speed
- Mark-and-Sweep GC with developer hinting (no surprises)

---

## Build and Run

**Prerequisites**
- Rust (Cargo)
- C++ Compiler (CMake)
- Python 3

**Quick Start**

Initialize the toolchain:

```bash
python scripts/Amberlink.py init
```

Compile your code:
```bash
python scripts/Amberlink.py build main.amb
```

Run it:
```bash
./bin/avm output.amc        # Linux/macOS
.\bin\avm.exe output.amc    # Windows
```

---

## What's Coming in 2.0

- Full OOP — classes, interfaces, inheritance
- Exception handling
- Generics
- Native backend (amber-native)
- IntelliJ and VS Code support via LSP

---

## License

MIT — do what you want, just don't be weird about it.

*Part of the [Sunlight Foundation](https://github.com/Sunlight-Foundation)*
