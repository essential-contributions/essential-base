//! A small crate that exports the [ASM_YAML] spec string and provides a
//! structured [Tree] model representing its deserialized form.

use serde::Deserialize;

mod de;
pub mod visit;

/// The raw YAML specification string.
pub const ASM_YAML: &str = include_str!("./../asm.yml");

/// The special name of the op group that describes the subset of operations
/// specific to constraint checker execution.
pub const CONSTRAINT_OP_NAME: &str = "Constraint";

/// Operations are laid out in a rose tree.
/// Nodes are ordered by their opcode, ensured during deserialisation.
#[derive(Debug)]
pub struct Tree(pub Vec<(String, Node)>);

/// Each node of the tree can be an operation, or another group.
#[derive(Debug)]
pub enum Node {
    Op(Op),
    Group(Group),
}

/// A group of related operations and subgroups.
#[derive(Debug, Deserialize)]
pub struct Group {
    pub description: String,
    #[serde(rename = "group")]
    pub tree: Tree,
}

/// A single operation.
#[derive(Debug, Deserialize)]
pub struct Op {
    pub opcode: u8,
    pub description: String,
    #[serde(default)]
    pub panics: Vec<String>,
    #[serde(default)]
    pub arg_bytes: u8,
    #[serde(default)]
    pub stack_in: Vec<String>,
    #[serde(default)]
    pub stack_out: StackOut,
}

/// The stack output of an operation, either fixed or dynamic (dependent on a `stack_in` value).
#[derive(Debug)]
pub enum StackOut {
    Fixed(Vec<String>),
    Dynamic(StackOutDynamic),
}

/// The stack output size is dynamic, dependent on a `stack_in` value.
#[derive(Debug, Deserialize)]
pub struct StackOutDynamic {
    pub elem: String,
    pub len: String,
}

impl Node {
    /// Get the opcode for the node.
    ///
    /// If the node is a group, this is the opcode of the first op.
    fn opcode(&self) -> u8 {
        match self {
            Self::Op(op) => op.opcode,
            Self::Group(group) => group.tree.first().unwrap().1.opcode(),
        }
    }
}

impl Default for StackOut {
    fn default() -> Self {
        Self::Fixed(vec![])
    }
}

impl std::ops::Deref for Tree {
    type Target = Vec<(String, Node)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Deserialize the top-level op tree from the YAML.
pub fn tree() -> Tree {
    serde_yaml::from_str::<Tree>(ASM_YAML)
        .expect("ASM_YAML is a const and should never fail to deserialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        // Panics internally on failure, but should never fail.
        let tree = tree();
        println!("{:#?}", tree);
    }

    #[test]
    fn test_no_duplicate_opcodes() {
        let tree = tree();
        let mut opcodes = std::collections::BTreeSet::new();
        super::visit::ops(&tree, &mut |name, op| {
            assert!(
                opcodes.insert(op.opcode),
                "ASM YAML must not contain duplicate opcodes. \
                Opcode `0x{:02X}` for {} already exists.",
                op.opcode,
                name.join(" "),
            );
        });
    }

    #[test]
    fn test_visit_ordered_by_opcode() {
        let tree = tree();
        let mut last_opcode = 0;
        super::visit::ops(&tree, &mut |_name, op| {
            assert!(
                last_opcode < op.opcode,
                "Visit functions are expected to visit ops in opcode order.\n  \
                last opcode: `0x{last_opcode:02X}`\n  \
                this opcode: `0x{:02X}`",
                op.opcode
            );
            last_opcode = op.opcode;
        });
    }

    #[test]
    fn test_constraint_op_exists() {
        use super::CONSTRAINT_OP_NAME;
        let tree = tree();
        let mut exists = false;
        super::visit::groups(&tree, &mut |names, _| {
            exists |= names.last().unwrap() == CONSTRAINT_OP_NAME
        });
        assert!(
            exists,
            "The `Constraint` op is a special operation that is expected to exist and \
            is used to distinguish between constraint and state read ops. Hitting this \
            error implies the ASM yaml has been refactored or the Constraint name may \
            have changed. If so, the `CONSTRAINT_OP_NAME` should be updated.",
        );
    }
}
