//! Items to assist with traversal of the operation tree and visiting groups/ops.

use crate::{Group, Node, Op, Tree};

/// Recursively visit all group nodes within the op tree that pass the given
/// predicate in depth-first visit order. The first argument is the stack of
/// nested names (e.g. `[Constraint, Alu]`) and is guaranteed to be non-empty.
pub fn groups_filtered(
    tree: &Tree,
    pred: &impl Fn(&str) -> bool,
    f: &mut impl FnMut(&[String], &Group),
) {
    groups_filtered_recurse(tree, pred, &mut vec![], f)
}

/// Exposed, recursive implementation of `groups_filtered`.
pub fn groups_filtered_recurse(
    tree: &Tree,
    pred: &impl Fn(&str) -> bool,
    names: &mut Vec<String>,
    f: &mut impl FnMut(&[String], &Group),
) {
    for (name, node) in tree.iter() {
        if let Node::Group(g) = node {
            if pred(name) {
                names.push(name.to_string());
                f(names, g);
                groups_filtered_recurse(&g.tree, pred, names, f);
                names.pop();
            }
        }
    }
}

/// Recursively visit all group nodes within the op tree in depth-first visit order.
pub fn groups(tree: &Tree, f: &mut impl FnMut(&[String], &Group)) {
    groups_filtered(tree, &|_| true, f)
}

/// Recursively visit all operations in order of their opcode, where the first argument to
/// the given function provides the fully nested name.
pub fn ops_filtered(tree: &Tree, pred: impl Fn(&str) -> bool, f: &mut impl FnMut(&[String], &Op)) {
    ops_filtered_recurse(tree, &pred, &mut vec![], f)
}

/// The main implementation of `ops_filtered`, but with a `names` argument to
/// enable recursion.
pub fn ops_filtered_recurse(
    tree: &Tree,
    pred: &impl Fn(&str) -> bool,
    names: &mut Vec<String>,
    f: &mut impl FnMut(&[String], &Op),
) {
    for (name, node) in tree.iter() {
        if pred(name) {
            names.push(name.to_string());
            match node {
                Node::Group(g) => ops_filtered_recurse(&g.tree, pred, names, f),
                Node::Op(op) => f(names, op),
            }
            names.pop();
        }
    }
}

/// Recursively visit all operations in order of their opcode, where the first argument to
/// the given function provides the fully nested name.
pub fn ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    ops_filtered(tree, |_| true, f)
}

/// Recursively visit only the sebset of op groups related to constraint execution.
pub fn constraint_groups(tree: &Tree, f: &mut impl FnMut(&[String], &Group)) {
    // Find the constraint group and only apply `f` to it and its children.
    groups(tree, &mut |name, group| {
        if name.last().unwrap() == crate::CONSTRAINT_GROUP_NAME {
            f(name, group);
            groups(&group.tree, f);
        }
    });
}

/// Recursively visit only the sebset of operations related to constraint execution.
pub fn constraint_ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    // Find the constraint group and only apply `f` to it and its children.
    groups(tree, &mut |names, group| {
        let name = names.last().unwrap();
        if name == crate::CONSTRAINT_GROUP_NAME {
            let mut names = vec![name.to_string()];
            ops_filtered_recurse(&group.tree, &|_| true, &mut names, f);
        }
    });
}

/// The predicate used to ensure only state-read-execution-specific groups and
/// ops are visited.
fn state_read_pred(name: &str) -> bool {
    name != crate::CONSTRAINT_GROUP_NAME
}

/// Recursively visit all op groups related solely to state read execution,
/// ignoring those that also appear in constraint execution. This is useful for
/// creating items specific to state read execution.
pub fn state_read_groups(tree: &Tree, f: &mut impl FnMut(&[String], &Group)) {
    groups_filtered(tree, &state_read_pred, f)
}

/// Recursively visit all ops related solely to state read execution, ignoring
/// those that also appear in constraint execution. This is useful for creating
/// items specific to state read execution.
pub fn state_read_ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    ops_filtered(tree, state_read_pred, f)
}
