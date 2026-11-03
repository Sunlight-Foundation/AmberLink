// amber-core/src/codegen/llvm_backend.rs
// LLVM AOT backend (Phase 6).
//
// Lowers Amberlink IR (the decoded .amc program) to textual LLVM IR, which
// `clang`/`llc` turn into a native executable. No LLVM libraries are linked
// into the compiler: the output is plain text.
//
// Value model
//   Every Amberlink value is a tagged `%Value = { i32 tag, i32 bits }`, the same
//   shape as the VM's Value. Common integer/float arithmetic and comparisons
//   lower directly to LLVM instructions; uncommon tagged-value cases and
//   operations such as printing use `runtime/amb_rt.c` for exact VM semantics
//   and error messages. Values are only ever passed by pointer.
//
// Operand stack
//   The VM's operand stack becomes a per-function `alloca` array. Stack depth is
//   tracked statically while lowering, so every push/pop is a fixed slot. Memory
//   based, so no phi nodes are needed across control flow. A function's args
//   occupy its first slots and double as locals, exactly like the VM's frame.
//
// Functions
//   The emitter lays function bodies inline behind a `jump` over them. This
//   backend recovers them (a top-level jump whose skipped range ends in
//   `return`) and emits each as `void @amb_fn_<entry>(ptr ret, ptr args)`.
//
// Supported: arithmetic, comparisons, strings (concat/compare/print), globals,
//   functions and calls, if/while/match (jumps), null literal and null checks.
// Not yet supported (reported as a compile error, never silently stubbed):
//   classes/fields, lists, arrays, native calls, spawn, try/throw.

use crate::codegen::ir::{IrInstr, IrProgram};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

const T_INT: i32 = 0;
const T_FLOAT: i32 = 1;
const T_BOOL: i32 = 2;
const T_CHAR: i32 = 3;
const T_STR: i32 = 5;
const T_NULL: i32 = 6;

struct Gen<'a> {
    consts: &'a [String],
    /// Offset of each instruction's successor in the whole program.
    next: HashMap<usize, usize>,
    body: String,
    tmp: usize,
    dead: usize,
    depth: i32,
    max_depth: i32,
    terminated: bool,
    depth_at: HashMap<usize, i32>,
    referenced: BTreeSet<usize>,
    emitted: HashSet<usize>,
    errors: Vec<String>,
}

impl<'a> Gen<'a> {
    fn t(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }

    fn line(&mut self, s: &str) {
        self.body.push_str("  ");
        self.body.push_str(s);
        self.body.push('\n');
    }

    /// Pointer to operand-stack slot `idx` (also grows the frame's max depth).
    fn slot(&mut self, idx: i32) -> String {
        if idx < 0 {
            self.errors
                .push(format!("internal: negative stack slot {}", idx));
            return "%stk".into();
        }
        if idx + 1 > self.max_depth {
            self.max_depth = idx + 1;
        }
        let n = self.t();
        self.line(&format!("{n} = getelementptr %Value, ptr %stk, i32 {idx}"));
        n
    }

    fn set_value(&mut self, slot: &str, tag: i32, bits: i32) {
        let tp = self.t();
        let bp = self.t();
        self.line(&format!(
            "{tp} = getelementptr %Value, ptr {slot}, i32 0, i32 0"
        ));
        self.line(&format!("store i32 {tag}, ptr {tp}"));
        self.line(&format!(
            "{bp} = getelementptr %Value, ptr {slot}, i32 0, i32 1"
        ));
        self.line(&format!("store i32 {bits}, ptr {bp}"));
    }

    fn copy_value(&mut self, dst: &str, src: &str) {
        let v = self.t();
        self.line(&format!("{v} = load %Value, ptr {src}"));
        self.line(&format!("store %Value {v}, ptr {dst}"));
    }

    fn push_const(&mut self, tag: i32, bits: i32) {
        let d = self.depth;
        let s = self.slot(d);
        self.set_value(&s, tag, bits);
        self.depth += 1;
    }

    fn need(&mut self, n: i32, what: &str) -> bool {
        if self.depth < n {
            self.errors
                .push(format!("internal: stack underflow lowering {}", what));
            self.depth = n;
            return false;
        }
        true
    }

    fn numeric_binary(&mut self, func: &str, int_op: &str, float_op: &str, division: bool) {
        if !self.need(2, func) {
            return;
        }
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        let id = self.tmp + 1;
        self.tmp = id;

        let atp = self.t();
        let btp = self.t();
        let atag = self.t();
        let btag = self.t();
        let abp = self.t();
        let bbp = self.t();
        let abits = self.t();
        let bbits = self.t();
        let a_is_int = self.t();
        let b_is_int = self.t();
        let both_int = self.t();
        self.line(&format!(
            "{atp} = getelementptr %Value, ptr {a}, i32 0, i32 0"
        ));
        self.line(&format!(
            "{btp} = getelementptr %Value, ptr {b}, i32 0, i32 0"
        ));
        self.line(&format!("{atag} = load i32, ptr {atp}"));
        self.line(&format!("{btag} = load i32, ptr {btp}"));
        self.line(&format!(
            "{abp} = getelementptr %Value, ptr {a}, i32 0, i32 1"
        ));
        self.line(&format!(
            "{bbp} = getelementptr %Value, ptr {b}, i32 0, i32 1"
        ));
        self.line(&format!("{abits} = load i32, ptr {abp}"));
        self.line(&format!("{bbits} = load i32, ptr {bbp}"));
        self.line(&format!("{a_is_int} = icmp eq i32 {atag}, {T_INT}"));
        self.line(&format!("{b_is_int} = icmp eq i32 {btag}, {T_INT}"));
        self.line(&format!("{both_int} = and i1 {a_is_int}, {b_is_int}"));
        self.line(&format!(
            "br i1 {both_int}, label %N{id}_int, label %N{id}_check_float"
        ));

        self.body.push_str(&format!("N{id}_int:\n"));
        if division {
            let zero = self.t();
            let is_min = self.t();
            let is_neg_one = self.t();
            let overflow = self.t();
            let slow = self.t();
            self.line(&format!("{zero} = icmp eq i32 {bbits}, 0"));
            self.line(&format!("{is_min} = icmp eq i32 {abits}, -2147483648"));
            self.line(&format!("{is_neg_one} = icmp eq i32 {bbits}, -1"));
            self.line(&format!("{overflow} = and i1 {is_min}, {is_neg_one}"));
            self.line(&format!("{slow} = or i1 {zero}, {overflow}"));
            self.line(&format!(
                "br i1 {slow}, label %N{id}_fallback, label %N{id}_int_fast"
            ));
            self.body.push_str(&format!("N{id}_int_fast:\n"));
        }
        let int_result = self.t();
        self.line(&format!("{int_result} = {int_op} i32 {abits}, {bbits}"));
        let int_out_tag = self.t();
        let int_out_bits = self.t();
        self.line(&format!(
            "{int_out_tag} = getelementptr %Value, ptr {a}, i32 0, i32 0"
        ));
        self.line(&format!(
            "{int_out_bits} = getelementptr %Value, ptr {a}, i32 0, i32 1"
        ));
        self.line(&format!("store i32 {T_INT}, ptr {int_out_tag}"));
        self.line(&format!("store i32 {int_result}, ptr {int_out_bits}"));
        self.line(&format!("br label %N{id}_done"));

        self.body.push_str(&format!("N{id}_check_float:\n"));
        let a_is_float = self.t();
        let b_is_float = self.t();
        let both_float = self.t();
        self.line(&format!("{a_is_float} = icmp eq i32 {atag}, {T_FLOAT}"));
        self.line(&format!("{b_is_float} = icmp eq i32 {btag}, {T_FLOAT}"));
        self.line(&format!("{both_float} = and i1 {a_is_float}, {b_is_float}"));
        self.line(&format!(
            "br i1 {both_float}, label %N{id}_float, label %N{id}_fallback"
        ));

        self.body.push_str(&format!("N{id}_float:\n"));
        let af = self.t();
        let bf = self.t();
        self.line(&format!("{af} = bitcast i32 {abits} to float"));
        self.line(&format!("{bf} = bitcast i32 {bbits} to float"));
        if division {
            let zero = self.t();
            self.line(&format!("{zero} = fcmp oeq float {bf}, 0.000000e+00"));
            self.line(&format!(
                "br i1 {zero}, label %N{id}_fallback, label %N{id}_float_fast"
            ));
            self.body.push_str(&format!("N{id}_float_fast:\n"));
        }
        let float_result = self.t();
        let float_bits = self.t();
        self.line(&format!("{float_result} = {float_op} float {af}, {bf}"));
        self.line(&format!(
            "{float_bits} = bitcast float {float_result} to i32"
        ));
        let float_out_tag = self.t();
        let float_out_bits = self.t();
        self.line(&format!(
            "{float_out_tag} = getelementptr %Value, ptr {a}, i32 0, i32 0"
        ));
        self.line(&format!(
            "{float_out_bits} = getelementptr %Value, ptr {a}, i32 0, i32 1"
        ));
        self.line(&format!("store i32 {T_FLOAT}, ptr {float_out_tag}"));
        self.line(&format!("store i32 {float_bits}, ptr {float_out_bits}"));
        self.line(&format!("br label %N{id}_done"));

        self.body.push_str(&format!("N{id}_fallback:\n"));
        self.line(&format!("call void @{func}(ptr {a}, ptr {a}, ptr {b})"));
        self.line(&format!("br label %N{id}_done"));
        self.body.push_str(&format!("N{id}_done:\n"));
        self.depth -= 1;
    }

    fn compare_fast(&mut self, op: i32, int_pred: &str, float_pred: &str) {
        if !self.need(2, "compare") {
            return;
        }
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        let id = self.tmp + 1;
        self.tmp = id;

        let atp = self.t();
        let btp = self.t();
        let atag = self.t();
        let btag = self.t();
        let abp = self.t();
        let bbp = self.t();
        let abits = self.t();
        let bbits = self.t();
        let a_is_int = self.t();
        let b_is_int = self.t();
        let both_int = self.t();
        self.line(&format!(
            "{atp} = getelementptr %Value, ptr {a}, i32 0, i32 0"
        ));
        self.line(&format!(
            "{btp} = getelementptr %Value, ptr {b}, i32 0, i32 0"
        ));
        self.line(&format!("{atag} = load i32, ptr {atp}"));
        self.line(&format!("{btag} = load i32, ptr {btp}"));
        self.line(&format!(
            "{abp} = getelementptr %Value, ptr {a}, i32 0, i32 1"
        ));
        self.line(&format!(
            "{bbp} = getelementptr %Value, ptr {b}, i32 0, i32 1"
        ));
        self.line(&format!("{abits} = load i32, ptr {abp}"));
        self.line(&format!("{bbits} = load i32, ptr {bbp}"));
        self.line(&format!("{a_is_int} = icmp eq i32 {atag}, {T_INT}"));
        self.line(&format!("{b_is_int} = icmp eq i32 {btag}, {T_INT}"));
        self.line(&format!("{both_int} = and i1 {a_is_int}, {b_is_int}"));
        self.line(&format!(
            "br i1 {both_int}, label %C{id}_int, label %C{id}_check_float"
        ));

        self.body.push_str(&format!("C{id}_int:\n"));
        let int_cmp = self.t();
        let int_bits = self.t();
        self.line(&format!("{int_cmp} = icmp {int_pred} i32 {abits}, {bbits}"));
        self.line(&format!("{int_bits} = zext i1 {int_cmp} to i32"));
        self.store_bool_result(&a, &int_bits);
        self.line(&format!("br label %C{id}_done"));

        self.body.push_str(&format!("C{id}_check_float:\n"));
        let a_is_float = self.t();
        let b_is_float = self.t();
        let both_float = self.t();
        self.line(&format!("{a_is_float} = icmp eq i32 {atag}, {T_FLOAT}"));
        self.line(&format!("{b_is_float} = icmp eq i32 {btag}, {T_FLOAT}"));
        self.line(&format!("{both_float} = and i1 {a_is_float}, {b_is_float}"));
        self.line(&format!(
            "br i1 {both_float}, label %C{id}_float, label %C{id}_fallback"
        ));

        self.body.push_str(&format!("C{id}_float:\n"));
        let af = self.t();
        let bf = self.t();
        let float_cmp = self.t();
        let float_bits = self.t();
        self.line(&format!("{af} = bitcast i32 {abits} to float"));
        self.line(&format!("{bf} = bitcast i32 {bbits} to float"));
        self.line(&format!("{float_cmp} = fcmp {float_pred} float {af}, {bf}"));
        self.line(&format!("{float_bits} = zext i1 {float_cmp} to i32"));
        self.store_bool_result(&a, &float_bits);
        self.line(&format!("br label %C{id}_done"));

        self.body.push_str(&format!("C{id}_fallback:\n"));
        self.line(&format!(
            "call void @amb_cmp(ptr {a}, ptr {a}, ptr {b}, i32 {op})"
        ));
        self.line(&format!("br label %C{id}_done"));
        self.body.push_str(&format!("C{id}_done:\n"));
        self.depth -= 1;
    }

    fn store_bool_result(&mut self, slot: &str, bits: &str) {
        let tag_ptr = self.t();
        let bits_ptr = self.t();
        self.line(&format!(
            "{tag_ptr} = getelementptr %Value, ptr {slot}, i32 0, i32 0"
        ));
        self.line(&format!(
            "{bits_ptr} = getelementptr %Value, ptr {slot}, i32 0, i32 1"
        ));
        self.line(&format!("store i32 {T_BOOL}, ptr {tag_ptr}"));
        self.line(&format!("store i32 {bits}, ptr {bits_ptr}"));
    }

    fn jump_to(&mut self, target: usize) {
        self.referenced.insert(target);
        self.depth_at.insert(target, self.depth);
        self.line(&format!("br label %L{target}"));
        self.terminated = true;
    }

    fn global_ptr(&mut self, i: i32) -> String {
        let g = self.t();
        self.line(&format!(
            "{g} = getelementptr %Value, ptr @amb_globals, i32 {i}"
        ));
        g
    }

    /// Lowers one function body (or the top-level program when `is_main`).
    /// Returns the complete LLVM text for it.
    fn function(
        &mut self,
        instrs: &[&(usize, IrInstr)],
        name: &str,
        argc: i32,
        is_main: bool,
    ) -> String {
        self.body.clear();
        self.depth = argc;
        self.max_depth = argc;
        self.terminated = false;
        self.depth_at.clear();
        self.referenced.clear();
        self.emitted.clear();

        // Block leaders: jump targets and the instruction after any terminator.
        let mut leaders: BTreeSet<usize> = BTreeSet::new();
        for (i, (_, ins)) in instrs.iter().enumerate() {
            match ins {
                IrInstr::Jump { target } | IrInstr::JumpIfFalse { target } => {
                    leaders.insert(*target);
                    if let Some(n) = instrs.get(i + 1) {
                        leaders.insert(n.0);
                    }
                }
                IrInstr::Return | IrInstr::Halt => {
                    if let Some(n) = instrs.get(i + 1) {
                        leaders.insert(n.0);
                    }
                }
                _ => {}
            }
        }

        self.body.push_str("B0:\n");

        for (off, ins) in instrs.iter().map(|p| (p.0, &p.1)) {
            if leaders.contains(&off) {
                if !self.terminated {
                    self.depth_at.entry(off).or_insert(self.depth);
                    self.referenced.insert(off);
                    self.line(&format!("br label %L{off}"));
                }
                if let Some(d) = self.depth_at.get(&off) {
                    self.depth = *d;
                }
                self.body.push_str(&format!("L{off}:\n"));
                self.emitted.insert(off);
                self.terminated = false;
            } else if self.terminated {
                // Unreachable code after a terminator: give it a block so the IR stays valid.
                self.dead += 1;
                let d = self.dead;
                self.body.push_str(&format!("D{d}:\n"));
                self.terminated = false;
            }
            self.instr(off, ins, is_main);
        }

        // Jumps may target the end of the region (e.g. the end of the program).
        let missing: Vec<usize> = self
            .referenced
            .iter()
            .copied()
            .filter(|t| !self.emitted.contains(t))
            .collect();
        for (k, t) in missing.iter().enumerate() {
            if !self.terminated {
                self.line(&format!("br label %L{t}"));
            }
            self.body.push_str(&format!("L{t}:\n"));
            if let Some(d) = self.depth_at.get(t) {
                self.depth = *d;
            }
            self.terminated = false;
            let _ = k;
        }
        if !self.terminated {
            if is_main {
                self.line("ret i32 0");
            } else {
                self.line("ret void");
            }
        }

        let frame = self.max_depth.max(1);
        let mut out = String::new();
        if is_main {
            writeln!(out, "define i32 @{name}() {{").unwrap();
        } else {
            writeln!(out, "define void @{name}(ptr %ret, ptr %args) {{").unwrap();
        }
        writeln!(out, "entry:").unwrap();
        writeln!(out, "  %stk = alloca [{frame} x %Value]").unwrap();
        if is_main {
            writeln!(
                out,
                "  call void @amb_init(ptr @amb_consts, i32 {})",
                self.consts.len()
            )
            .unwrap();
        } else {
            // Copy the incoming args into the frame's first slots.
            for i in 0..argc {
                writeln!(out, "  %ap{i} = getelementptr %Value, ptr %args, i32 {i}").unwrap();
                writeln!(out, "  %av{i} = load %Value, ptr %ap{i}").unwrap();
                writeln!(out, "  %fp{i} = getelementptr %Value, ptr %stk, i32 {i}").unwrap();
                writeln!(out, "  store %Value %av{i}, ptr %fp{i}").unwrap();
            }
        }
        writeln!(out, "  br label %B0").unwrap();
        out.push_str(&self.body);
        writeln!(out, "}}\n").unwrap();
        out
    }

    fn instr(&mut self, off: usize, ins: &IrInstr, is_main: bool) {
        match ins {
            IrInstr::Halt => {
                if is_main {
                    self.line("ret i32 0");
                } else {
                    self.line("ret void");
                }
                self.terminated = true;
            }
            IrInstr::Push(v) => self.push_const(T_INT, *v),
            IrInstr::PushFloat(v) => self.push_const(T_FLOAT, v.to_bits() as i32),
            IrInstr::PushBool(v) => self.push_const(T_BOOL, i32::from(*v)),
            IrInstr::PushChar(v) => self.push_const(T_CHAR, *v),
            IrInstr::PushNull => self.push_const(T_NULL, 0),
            IrInstr::LoadConst(i) => {
                if (*i as usize) >= self.consts.len() {
                    self.errors
                        .push(format!("@{off}: constant index {} out of range", i));
                }
                self.push_const(T_STR, *i)
            }
            IrInstr::LoadGlobal(i) => {
                let d = self.depth;
                let dst = self.slot(d);
                let g = self.global_ptr(*i);
                self.copy_value(&dst, &g);
                self.depth += 1;
            }
            IrInstr::StoreGlobal(i) => {
                if !self.need(1, "store_global") {
                    return;
                }
                let src = self.slot(self.depth - 1);
                let g = self.global_ptr(*i);
                self.copy_value(&g, &src);
                self.depth -= 1;
            }
            IrInstr::LoadLocal(i) => {
                let d = self.depth;
                let dst = self.slot(d);
                let src = self.slot(*i);
                self.copy_value(&dst, &src);
                self.depth += 1;
            }
            IrInstr::StoreLocal(i) => {
                if !self.need(1, "store_local") {
                    return;
                }
                let src = self.slot(self.depth - 1);
                self.depth -= 1;
                let dst = self.slot(*i);
                self.copy_value(&dst, &src);
                // Like the VM, storing past the top of the stack grows the frame.
                if *i + 1 > self.depth {
                    self.depth = *i + 1;
                }
            }
            IrInstr::Add => self.numeric_binary("amb_add", "add", "fadd", false),
            IrInstr::Sub => self.numeric_binary("amb_sub", "sub", "fsub", false),
            IrInstr::Mul => self.numeric_binary("amb_mul", "mul", "fmul", false),
            IrInstr::Div => self.numeric_binary("amb_div", "sdiv", "fdiv", true),
            IrInstr::Less => self.compare_fast(0, "slt", "olt"),
            IrInstr::Greater => self.compare_fast(1, "sgt", "ogt"),
            IrInstr::Equal => self.compare_fast(2, "eq", "oeq"),
            IrInstr::NotEqual => self.compare_fast(3, "ne", "une"),
            IrInstr::LessEqual => self.compare_fast(4, "sle", "ole"),
            IrInstr::GreaterEqual => self.compare_fast(5, "sge", "oge"),
            IrInstr::Pop => {
                if self.need(1, "pop") {
                    self.depth -= 1;
                }
            }
            IrInstr::Print => {
                if !self.need(1, "print") {
                    return;
                }
                let s = self.slot(self.depth - 1);
                self.line(&format!("call void @amb_print(ptr {s})"));
                self.depth -= 1;
            }
            IrInstr::CheckNull => {
                if !self.need(1, "check_null") {
                    return;
                }
                let s = self.slot(self.depth - 1);
                self.line(&format!("call void @amb_check_null(ptr {s})"));
            }
            IrInstr::Call { addr, argc } => {
                let argc = *argc as i32;
                if !self.need(argc, "call") {
                    return;
                }
                let base = self.depth - argc;
                let slot = self.slot(base);
                // The callee copies its args into its own frame before writing
                // the result, so result and args can share a slot.
                self.line(&format!("call void @amb_fn_{addr}(ptr {slot}, ptr {slot})"));
                self.depth = base + 1;
            }
            IrInstr::Return => {
                let src = if self.depth >= 1 {
                    self.slot(self.depth - 1)
                } else {
                    let s = self.slot(0);
                    self.set_value(&s, T_NULL, 0);
                    s
                };
                self.copy_value("%ret", &src);
                self.line("ret void");
                self.terminated = true;
            }
            IrInstr::Jump { target } => self.jump_to(*target),
            IrInstr::JumpIfFalse { target } => {
                if !self.need(1, "jump_if_false") {
                    return;
                }
                let s = self.slot(self.depth - 1);
                self.depth -= 1;
                let r = self.t();
                let c = self.t();
                self.line(&format!("{r} = call i32 @amb_is_false(ptr {s})"));
                self.line(&format!("{c} = icmp ne i32 {r}, 0"));
                self.referenced.insert(*target);
                self.depth_at.insert(*target, self.depth);
                // The fall-through block is the next instruction (always a leader).
                let next = *self.next.get(&off).unwrap_or(&(off + 5));
                self.referenced.insert(next);
                self.depth_at.insert(next, self.depth);
                self.line(&format!("br i1 {c}, label %L{target}, label %L{next}"));
                self.terminated = true;
            }
            other => {
                self.errors.push(format!(
                    "@{off}: `{}` is not supported by the LLVM AOT backend yet",
                    crate::codegen::ir::format_instr(other, self.consts)
                ));
            }
        }
    }
}

fn escape_c_string(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        if (0x20..0x7f).contains(&b) && b != b'"' && b != b'\\' {
            out.push(b as char);
        } else {
            write!(out, "\\{:02X}", b).unwrap();
        }
    }
    out
}

/// Emits LLVM IR for a decoded program. `constants` is the string pool.
pub fn emit_llvm(prog: &IrProgram, constants: &[String]) -> Result<String, Vec<String>> {
    // Recover function ranges: a top-level jump whose skipped range ends in `return`.
    let mut ranges: Vec<(usize, usize)> = Vec::new(); // [entry, end)
    let mut i = 0;
    while i < prog.len() {
        if let (off, IrInstr::Jump { target }) = &prog[i] {
            if *target > *off {
                let inner: Vec<&(usize, IrInstr)> = prog
                    .iter()
                    .filter(|(o, _)| *o > *off && *o < *target)
                    .collect();
                if let Some((_, IrInstr::Return)) = inner.last() {
                    ranges.push((inner[0].0, *target));
                    while i < prog.len() && prog[i].0 < *target {
                        i += 1;
                    }
                    continue;
                }
            }
        }
        i += 1;
    }

    // Arity comes from call sites.
    let mut argc_of: HashMap<i32, u8> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();
    for (off, ins) in prog {
        if let IrInstr::Call { addr, argc } = ins {
            match argc_of.get(addr) {
                Some(prev) if prev != argc => errors.push(format!(
                    "@{off}: function @{addr} called with {} and {} args",
                    prev, argc
                )),
                _ => {
                    argc_of.insert(*addr, *argc);
                }
            }
            if !ranges.iter().any(|(e, _)| *e as i32 == *addr) {
                errors.push(format!(
                    "@{off}: call target @{addr} is not a recognised function entry"
                ));
            }
        }
    }

    let global_count = prog
        .iter()
        .filter_map(|(_, ins)| match ins {
            IrInstr::LoadGlobal(i) | IrInstr::StoreGlobal(i) => Some(*i + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .max(1);

    let mut next: HashMap<usize, usize> = HashMap::new();
    for w in prog.windows(2) {
        next.insert(w[0].0, w[1].0);
    }

    let mut gen = Gen {
        consts: constants,
        next,
        body: String::new(),
        tmp: 0,
        dead: 0,
        depth: 0,
        max_depth: 0,
        terminated: false,
        depth_at: HashMap::new(),
        referenced: BTreeSet::new(),
        emitted: HashSet::new(),
        errors,
    };

    let in_range = |off: usize| ranges.iter().any(|(s, e)| off >= *s && off < *e);

    let mut funcs = String::new();
    for (entry, end) in &ranges {
        let body: Vec<&(usize, IrInstr)> = prog
            .iter()
            .filter(|(o, _)| *o >= *entry && *o < *end)
            .collect();
        let argc = *argc_of.get(&(*entry as i32)).unwrap_or(&0) as i32;
        funcs.push_str(&gen.function(&body, &format!("amb_fn_{entry}"), argc, false));
    }
    let main_body: Vec<&(usize, IrInstr)> = prog.iter().filter(|(o, _)| !in_range(*o)).collect();
    let main_fn = gen.function(&main_body, "main", 0, true);

    if !gen.errors.is_empty() {
        return Err(gen.errors);
    }

    let mut out = String::new();
    writeln!(out, "; Amberlink LLVM IR (amber-core LLVM AOT backend)").unwrap();
    writeln!(out, "%Value = type {{ i32, i32 }}\n").unwrap();
    for (i, s) in constants.iter().enumerate() {
        writeln!(
            out,
            "@amb_str{i} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            s.len() + 1,
            escape_c_string(s)
        )
        .unwrap();
    }
    let refs: Vec<String> = (0..constants.len())
        .map(|i| format!("ptr @amb_str{i}"))
        .collect();
    writeln!(
        out,
        "@amb_consts = private constant [{} x ptr] [{}]",
        constants.len(),
        refs.join(", ")
    )
    .unwrap();
    writeln!(
        out,
        "@amb_globals = internal global [{} x %Value] zeroinitializer\n",
        global_count
    )
    .unwrap();
    out.push_str("declare void @amb_init(ptr, i32)\n");
    for f in ["amb_add", "amb_sub", "amb_mul", "amb_div"] {
        writeln!(out, "declare void @{f}(ptr, ptr, ptr)").unwrap();
    }
    out.push_str("declare void @amb_cmp(ptr, ptr, ptr, i32)\n");
    out.push_str("declare i32 @amb_is_false(ptr)\n");
    out.push_str("declare void @amb_check_null(ptr)\n");
    out.push_str("declare void @amb_print(ptr)\n\n");
    out.push_str(&funcs);
    out.push_str(&main_fn);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::emit_llvm;
    use crate::codegen::ir::IrInstr;

    #[test]
    fn emits_a_valid_minimal_main_and_runtime_declarations() {
        let program = vec![
            (0, IrInstr::Push(7)),
            (5, IrInstr::Print),
            (6, IrInstr::Halt),
        ];
        let llvm = emit_llvm(&program, &[]).expect("supported IR should lower");
        assert!(llvm.contains("define i32 @main()"));
        assert!(llvm.contains("call void @amb_print"));
        assert!(llvm.contains("ret i32 0"));
    }

    #[test]
    fn emits_numeric_fast_paths_and_preserves_runtime_fallbacks() {
        let program = vec![
            (0, IrInstr::Push(7)),
            (5, IrInstr::Push(2)),
            (10, IrInstr::Add),
            (11, IrInstr::Print),
            (12, IrInstr::PushFloat(3.5)),
            (17, IrInstr::PushFloat(1.5)),
            (22, IrInstr::Sub),
            (23, IrInstr::Print),
            (24, IrInstr::Push(6)),
            (29, IrInstr::Push(2)),
            (34, IrInstr::Div),
            (35, IrInstr::Print),
            (36, IrInstr::Push(3)),
            (41, IrInstr::Push(4)),
            (46, IrInstr::Less),
            (47, IrInstr::Print),
            (48, IrInstr::Halt),
        ];
        let llvm = emit_llvm(&program, &[]).expect("numeric operations should lower");

        assert!(llvm.contains(" = add i32 "));
        assert!(llvm.contains(" = fsub float "));
        assert!(llvm.contains(" = sdiv i32 "));
        assert!(llvm.contains(" = fdiv float "));
        assert!(llvm.contains(" = icmp slt i32 "));
        assert!(llvm.contains(" = fcmp olt float "));
        assert!(llvm.contains("call void @amb_add"));
        assert!(llvm.contains("call void @amb_sub"));
        assert!(llvm.contains("call void @amb_div"));
        assert!(llvm.contains("call void @amb_cmp"));
        assert!(llvm.contains("icmp eq i32"));
        assert!(llvm.contains("-2147483648"));
    }

    #[test]
    fn floating_not_equal_uses_unordered_predicate_like_c_runtime() {
        let program = vec![
            (0, IrInstr::PushFloat(f32::NAN)),
            (5, IrInstr::PushFloat(1.0)),
            (10, IrInstr::NotEqual),
            (11, IrInstr::Halt),
        ];
        let llvm = emit_llvm(&program, &[]).expect("float comparison should lower");
        assert!(llvm.contains("fcmp une float"));
    }

    #[test]
    fn rejects_instructions_outside_the_documented_aot_subset() {
        let program = vec![(0, IrInstr::NewList), (1, IrInstr::Halt)];
        let errors = emit_llvm(&program, &[]).expect_err("unsupported instructions must fail");
        assert!(errors
            .iter()
            .any(|e| e.contains("not supported by the LLVM AOT backend")));
    }

    #[test]
    fn rejects_calls_to_unknown_function_entries() {
        let program = vec![
            (0, IrInstr::Push(1)),
            (5, IrInstr::Call { addr: 99, argc: 1 }),
            (10, IrInstr::Halt),
        ];
        let errors = emit_llvm(&program, &[]).expect_err("unknown call targets must fail");
        assert!(errors
            .iter()
            .any(|e| e.contains("call target @99 is not a recognised function entry")));
    }
}
