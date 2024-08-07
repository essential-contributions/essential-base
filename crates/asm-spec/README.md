
# Essential ASM Specification.

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![license][apache-badge]][apache-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/essential-asm-spec.svg
[crates-url]: https://crates.io/crates/essential-asm-spec
[docs-badge]: https://docs.rs/essential-asm-spec/badge.svg
[docs-url]: https://docs.rs/essential-asm-spec
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: LICENSE
[actions-badge]: https://github.com/essential-contributions/essential-base/workflows/ci/badge.svg
[actions-url]:https://github.com/essential-contributions/essential-base/actions

This crate parses the Essential ASM specification from YAML and provides a
structured model for deserializing and traversing the tree of operations.

The primary use is to assist in generating the official ASM declaration
and implementations, though is likely useful for other tooling based on the
essential ASM spec.

## Operation Declaration

Each operation is identified by a unique name and contains the following fields:

- `opcode`: A hexadecimal representation of the operation code, uniquely
  identifying the operation.
- `description`: A brief explanation of what the operation does.
- `panics` (optional): A list of reasons why the operation might cause the
  virtual machine to panic.
- `num_arg_bytes` (optional): Specifies the number of bytes expected as arguments
  for the operation.
- `stack_in`: Defines the inputs taken from the stack before operation
  execution. This is a list of symbolic identifiers representing the expected
  values. If `stack_in` is omitted, an empty list is assumed.
- `stack_out`: Describes the outputs pushed onto the stack after operation
  execution. The stack output can either be "fixed" or "dynamic".
  - *fixed*: Used when the number of items pushed to the stack is constant.
    Represented as a list of strings representing the output values.
  - *dynamic*: Used when the number of items pushed to the stack can vary.
    Represented as a mapping with the following fields:
    - The `elem` field is a symbolic identifier representing the output values.
    - The `len` field specifies which `stack_in` word the length is derived from.

**Examples**

```yaml
Push:
  opcode: 0x01
  description: Push one word onto the stack.
  num_arg_bytes: 8
  stack_out: [word]
```

```yaml
ReadWordRange:
  opcode: 0x60
  description: |
    Read a range of words from state starting at the key.

    Reads the state key and the number of words from the stack input.

    Returns the memory address at which the data was written via the stack output.
  panics:
    - Not enough memory allocated to store the read state.
  stack_in: [key_w0, key_w1, key_w2, key_w3, n_words]
  stack_out: [mem_addr]
```

## Operation Group

An operation group organizes related operations. It can include:
- description: A brief overview of the group's purpose.
- group: A mapping from names to operations (or other groups) within this group.

**Example**

```yaml
    Stack:
      description: Operations related to stack manipulation.
      group:
        # Push:
        # etc
```

## Multi-word Values

When a multi-word value (like a state key or an address) is read from the
stack, the most-significant bits are assumed to have been pushed to the stack
first.
