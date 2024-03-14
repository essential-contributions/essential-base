
# Essential ASM declaration.

This crate parses the Essential ASM declaration from YAML and generates a Rust
AST for use within the official Rust implementation.

## Operation Declaration

Each operation is identified by a unique name and contains the following fields:

- `opcode`: A hexadecimal representation of the operation code, uniquely
  identifying the operation.
- `description`: A brief explanation of what the operation does.
- `panics` (optional): A list of reasons why the operation might cause the
  virtual machine to panic.
- `arg_bytes` (optional): Specifies the number of bytes expected as arguments
  for the operation.
- `stack_in`: Defines the inputs taken from the stack before operation
  execution. This is a list of symbolic identifiers representing the expected
  values. If `stack_in` is omitted, an empty list is assumed.
- `stack_out`: Describes the outputs pushed onto the stack after operation
  execution. It can be marked as `!fixed` or `!dynamic` to indicate whether the
  output size is constant or variable.
  - `!fixed`: Used when the number of items pushed to the stack is constant.
    Followed by a list of symbolic identifiers representing the output values.
  - `!dynamic`: Used when the number of items pushed to the stack can vary.
    - The `elem` field is a symbolic identifier representing the output values.
    - The `len` field specifies which `stack_in` word the length is derived from.

**Examples**

```yaml
Push:
  opcode: 0x01
  description: Push one word onto the stack.
  arg_bytes: 8
  stack_out: !fixed [word]
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
  stack_out: !fixed [mem_addr]
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
