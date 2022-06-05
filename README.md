# language-greenhouse

[![CI](https://img.shields.io/github/workflow/status/femtomc/language-greenhouse/CI?style=for-the-badge)](https://github.com/femtomc/language-greenhouse/actions?query=workflow%3ACI)

This is a small collection of interpreters/compilers focused on the technique of compilation by staging.

For more information, please see [From definitional interpreters to native code through staging](https://femtomc.github.io/posts/from_definitional_interpreters_to_native_code_through_staging/).

In general, each language module (see below) is self-contained -- featuring a language definition, an interpreter (which implements a small step semantics for the language), a parser for that language (I haven't defined the grammar, but refer to tests), and a JIT compiler using [cranelift](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift).

## Languages

- [X] [Simple calculator language with an environment.](https://github.com/femtomc/language-greenhouse/tree/master/src/calc)
- [ ] Functions as first class objects.

---

<div align="center">
<sup>
Started by <a href="https://femtomc.github.io/">McCoy R. Becker</a>. All code is licensed under the <a href="LICENSE">MIT License</a>.
</sup>
</div>
