# Amberlink 🔶

A Java alternative that doesn't hate you.

Amberlink is a statically typed, multi-paradigm language built to give Java developers a cleaner home. Same familiar structure — classes, interfaces, strong typing — without the ceremony. Run on the Amber Virtual Machine or compile to a native binary. Your choice.

> **Status: v0.7 Beta — Phase 4 (Ecosystem & Tooling) in progress.**

---

## Why Amberlink?

Java is fine. We just think it can be better.

- No `public static void main` — code runs top to bottom
- No primitive vs object split — `int` is `int`, everywhere
- Dual-backend: AVM for managed execution, native for raw speed
- Mark-and-Sweep GC with developer hinting (no surprises)

---

## Build and Run

**Prerequisites**
- Rust (Cargo)
- C++ Compiler (CMake)
- `make` (ships with Git for Windows, macOS Xcode CLI tools, and all Linux distros)

**Quick Start**

Initialize the toolchain:

```bash
make init
```

Compile your code:
```bash
make build file=main.amb
```

Run it:
```bash
make run file=main.amc
```

Or run pre-compiled bytecode directly:
```bash
./bin/avm output.amc        # Linux/macOS
.\bin\avm.exe output.amc    # Windows
```

Scaffold a new project, run the regression suite, or rebuild on every change:
```bash
make new name=myproject
make test
make watch file=main.amb
```

---

## Roadmap

See [roadmap.md](roadmap.md) for the full development plan.

- **Phase 1** — Core Language (v0.1–0.3) ✅
- **Phase 2** — Data & Memory (v0.4) ✅
- **Phase 3** — Object-Oriented Programming (v0.6) ✅
- **Phase 4** — Ecosystem & Tooling (v0.7) 🔧 In Progress
- **Phase 5** — Performance (v0.8)
- **Phase 6** — Advanced Language Features (v0.9)
- **Phase 7** — JVM Integration (v1.0)
- **Phase 8** — Documentation & Community (v1.0)

---

## License

GNU General Public License v3.0 (GPL-3.0) — see the [LICENSE](LICENSE) file for the full text.

*Part of the [Sunlight Foundation](https://github.com/Sunlight-Foundation)*
