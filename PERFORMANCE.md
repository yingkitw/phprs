# phprs performance notes

This document describes **optimization techniques used in phprs** and **how to measure** behavior. It does **not** claim that phprs is faster than PHP unless you have **reproducible, documented** experiments for your workload.

## Evidence policy

- **Do not** treat language-level or design arguments as proof that phprs beats stock PHP. Those are *hypotheses* until measured.
- **Do** report numbers only when you can point to: PHP and phprs versions, exact commands or scripts, hardware/OS, iteration counts, and (for memory) how usage was measured.
- The in-tree benchmark helpers may compare against **placeholder baselines** for development; those are **not** substitutes for measuring real `php` on the same machine.

## What phprs implements (implementation facts)

These are features that exist in the codebase; they are **not** automatic proof of end-to-end speedup versus PHP.

### VM and execution

- **Dispatch**: Computed-style opcode dispatch (vs a large `match` on hot paths) to reduce branch overhead.
- **Memory for execution**: Pre-sized buffers where the code path expects known capacity.
- **Hot paths**: Some paths use `unsafe` only where invariants are documented and checked elsewhere—this is a tradeoff, not a guarantee of winning vs PHP.

### JIT, opcode cache, and function optimizer

- **JIT**: Hot-function detection (e.g. threshold around 100 invocations), limited native-style compilation paths for small/hot regions.
- **Opcode cache**: LRU-style caching with basic vs more aggressive optimization passes (constant folding, DCE, etc., as implemented).
- **Function optimizer**: Call-frequency tracking, complexity heuristics, selective inlining.

### Memory and strings

- **Pools / builders**: Small-object pooling and string builders aimed at reducing allocator churn for typical patterns.
- **String ops**: Interning, hashing, and concatenation paths tuned for the interpreter’s representation—not necessarily identical to PHP’s C implementation.

### Arrays

- **Optimized array type**: Alternate array representation for paths that use it; behavior and wins depend on workload.

## Why Rust *can* help (theory, not a phprs scorecard)

Rust’s toolchain and language model **can** enable optimizations (LLVM pipeline, monomorphization, ownership-based layout, fearless concurrency in **Rust** code). Whether that translates into **faster PHP programs in phprs** is **workload- and implementation-dependent** and must be measured.

Avoid comparing Rust’s LLVM backend to “GCC vs Clang” or other engines in this doc unless you cite **specific, controlled** measurements relevant to phprs.

## How to run what we have today

```bash
cargo build --release
cargo run --example performance_demo
```

If you add a dedicated benchmark binary or example, document the exact invocation here when you publish results.

For optimization introspection (API surface may change—refer to source):

```rust
use phprs::engine::{jit, opcode_cache, function_optimizer};

let jit_stats = jit::get_jit_compiler().get_stats();
let cache_stats = opcode_cache::get_opcode_cache().get_stats();
let func_stats = function_optimizer::get_function_optimizer().get_stats();
```

## Publishing a real phprs vs PHP comparison

When you have measured data, add a subsection under **Measured results** with:

1. **Versions**: `php -v`, phprs commit or release tag, Rust toolchain (`rustc --version`).
2. **Hardware/OS**: CPU model, RAM, OS build.
3. **Workload**: Minimal script or app under test; warm-up policy; iteration count.
4. **Metrics**: Wall time, CPU time if available, peak RSS (define tool: `time`, `perf`, `/usr/bin/time -l`, etc.).
5. **Raw output**: Logs or JSON paths (e.g. `benchmark_results.json`) committed or linked.

Until that exists, **do not** fill in speedup tables from estimates.

## Operational tips (release builds)

1. Use **`cargo build --release`** for meaningful timing.
2. If JIT is relevant, allow enough iterations for hot functions to cross the compile threshold before timing steady state.
3. Treat cache and optimizer statistics as **diagnostics**, not as proof of superiority over PHP.

## Future work

Possible directions (each needs measurement): PGO, explicit SIMD for hot kernels, parallel array ops, broader async I/O integration, WASM target, etc.

---

**Summary:** phprs includes many performance-oriented implementations; **claims relative to PHP require experiments**. This file should stay aligned with what is measured, not what sounds plausible.
