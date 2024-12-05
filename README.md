# Essential Base
[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-check.svg
[crates-url]: https://crates.io/crates/essential-check
[docs-badge]: https://docs.rs/essential-check/badge.svg
[docs-url]: https://docs.rs/essential-check
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-base/workflows/ci/badge.svg
[actions-url]:https://github.com/essential-contributions/essential-base/actions

The foundational crates that the rest of the Essential ecosystem is built upon.

### Core functionality
- [essential-check](./crates/check/README.md) Validate contracts and solutions. Read state and check constraints.
- [essential-vm](./crates/vm/README.md) Evaluate a predicate's programs.
- [essential-types](./crates/types/README.md) Base types used throughout the Essential ecosystem.

### Assembly
- [Assembly specification](./crates/asm-spec/asm.yml) The full list of operations that the Essential VMs support.
- [essential-asm-gen](./crates/asm-gen/README.md) Proc-macro for generating ASM types from spec.
- [essential-asm-spec](./crates/asm-spec/README.md) Parses the assembly yaml.
- [essential-asm](./crates/asm/README.md) Assembly operations for the Essential VM.

### Crypto
- [essential-hash](./crates/hash/README.md) Hashing functionality for the Essential ecosystem.
- [essential-sign](./crates/sign/README.md) Public key cryptography for the Essential ecosystem.

### Utilities
- [essential-lock](./crates/lock/README.md) Mutex that is safe to use in async contexts.
