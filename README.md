# sion-seed

Throwaway Stage-0 compiler for **Sion-0**, the bootstrap subset of Sion. Written in Rust, no dependencies.

It is deleted (archived) once the Sion compiler compiles itself to a byte-identical fixed point.

Rules:
- Only Sion-0 features. No generics, traits, lambdas, effect or ownership checking.
- No external crates, no LLVM, no linker. Emit PE, ELF and WASM bytes directly.
- Keep it under about 5k lines.

Progress: [x] lexer  [ ] parser  [ ] type check  [ ] x86-64 emit  [ ] WASM emit

Try: `cargo run -- lex ../sion-tests/tests/001_hello.sn`
