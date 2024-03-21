//! Some fns to assist with traversing the node tree to visit groups and ops.

use crate::{Group, Node, Op, Tree};

/// Recursively visit all group nodes within the op tree that pass the given
/// predicate in depth-first visit order.
pub(crate) fn groups_filtered(
    tree: &Tree,
    pred: &impl Fn(&str) -> bool,
    f: &mut impl FnMut(&str, &Group),
) {
    for (name, node) in tree.iter() {
        match node {
            Node::Group(g) => {
                if pred(name) {
                    f(name, g);
                    groups_filtered(&g.tree, pred, f);
                }
            }
            _ => (),
        }
    }
}

/// Recursively visit all group nodes within the op tree in depth-first visit order.
pub(crate) fn groups(tree: &Tree, f: &mut impl FnMut(&str, &Group)) {
    groups_filtered(tree, &|_| true, f)
}

/// Recursively visit all operations in order of their opcode, where the first argument to
/// the given function provides the fully nested name.
pub(crate) fn ops_filtered(
    tree: &Tree,
    pred: impl Fn(&str) -> bool,
    f: &mut impl FnMut(&[String], &Op),
) {
    ops_filtered_recurse(tree, &pred, &mut vec![], f)
}

/// The main implementation of `ops_filtered`, but with a `names` argument to
/// enable recursion.
fn ops_filtered_recurse(
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
pub(crate) fn ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    ops_filtered(tree, |_| true, f)
}

/// Recursively visit only the subset of op groups related to constraint execution.
pub(crate) fn constraint_groups(tree: &Tree, f: &mut impl FnMut(&str, &Group)) {
    // Find the constraint group and only apply `f` to it and its children.
    groups(tree, &mut |name, group| {
        if name == crate::CONSTRAINT_OP_NAME {
            f(name, group);
            groups(&group.tree, f);
        }
    });
}

/// Recursively visit only the subset of operations related to constraint execution.
pub(crate) fn constraint_ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    // Find the constraint group and only apply `f` to it and its children.
    groups(tree, &mut |name, group| {
        if name == crate::CONSTRAINT_OP_NAME {
            let mut names = vec![name.to_string()];
            ops_filtered_recurse(&group.tree, &|_| true, &mut names, f);
        }
    });
}

/// The predicate used to ensure only state-read-execution-specific groups and
/// ops are visited.
fn state_read_pred(name: &str) -> bool {
    name != crate::CONSTRAINT_OP_NAME
}

/// Recursively visit all op groups related solely to state read execution,
/// ignoring those that also appear in constraint execution. This is useful for
/// creating items specific to state read execution.
pub(crate) fn state_read_groups(tree: &Tree, f: &mut impl FnMut(&str, &Group)) {
    groups_filtered(tree, &state_read_pred, f)
}

/// Recursively visit all ops related solely to state read execution, ignoring
/// those that also appear in constraint execution. This is useful for creating
/// items specific to state read execution.
pub(crate) fn state_read_ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    ops_filtered(tree, &state_read_pred, f)
}
