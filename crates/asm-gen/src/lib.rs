use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use serde::Deserialize;
use syn::{punctuated::Punctuated, token::Comma};

mod de;
mod visit;

const ASM_YAML: &str = include_str!("./../asm.yml");

/// The special name of the op group that describes the subset of operations
/// specific to constraint checker execution.
const CONSTRAINT_OP_NAME: &str = "Constraint";

/// Operations are laid out in a rose tree.
/// Nodes are ordered by their opcode, ensured during deserialisation.
#[derive(Debug)]
struct Tree(Vec<(String, Node)>);

/// Each node of the tree can be an operation, or another group.
#[derive(Debug)]
enum Node {
    Op(Op),
    Group(Group),
}

/// A group of related operations and subgroups.
#[derive(Debug, Deserialize)]
struct Group {
    description: String,
    #[serde(rename = "group")]
    tree: Tree,
}

/// A single operation.
#[derive(Debug, Deserialize)]
struct Op {
    opcode: u8,
    description: String,
    #[serde(default)]
    panics: Vec<String>,
    #[serde(default)]
    arg_bytes: u8,
    #[serde(default)]
    stack_in: Vec<String>,
    #[serde(default)]
    stack_out: StackOut,
}

/// The stack output of an operation, either fixed or dynamic (dependent on a `stack_in` value).
#[derive(Debug)]
enum StackOut {
    Fixed(Vec<String>),
    Dynamic(StackOutDynamic),
}

/// The stack output size is dynamic, dependent on a `stack_in` value.
#[derive(Debug, Deserialize)]
struct StackOutDynamic {
    elem: String,
    len: String,
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

/// Document the required bytecode arguments to the operation.
fn bytecode_arg_docs(arg_bytes: u8) -> String {
    if arg_bytes == 0 {
        return String::new();
    }
    let word_size = std::mem::size_of::<essential_types::Word>();
    assert_eq!(
        arg_bytes as usize % word_size,
        0,
        "doc gen currently only supports arguments that are a multiple of the word size"
    );
    let arg_words = arg_bytes as usize / word_size;
    format!(
        "## Bytecode Argument\nThis operation expects a {arg_bytes}-byte \
        ({arg_words}-word) argument following its opcode within bytecode."
    )
}

/// Generate an Op's stack-input docstring.
fn stack_in_docs(stack_in: &[String]) -> String {
    if stack_in.is_empty() {
        String::new()
    } else {
        format!("## Stack Input\n`[{}]`\n", stack_in.join(", "))
    }
}

/// Generate an Op's stack-output docstring.
fn stack_out_docs(stack_out: &StackOut) -> String {
    match stack_out {
        StackOut::Fixed(words) if words.is_empty() => String::new(),
        StackOut::Fixed(words) => {
            format!("## Stack Output\n`[{}]`\n", words.join(", "))
        }
        StackOut::Dynamic(out) => {
            format!(
                "## Stack Output\n`[{}, ...]`\nThe stack output length depends on the \
                value of the `{}` stack input word.\n",
                out.elem, out.len
            )
        }
    }
}

/// Generate an Op's panic reason docstring.
fn panic_docs(panic_reasons: &[String]) -> String {
    if panic_reasons.is_empty() {
        String::new()
    } else {
        let mut docs = "## Panics\n".to_string();
        panic_reasons
            .iter()
            .for_each(|reason| docs.push_str(&format!("- {reason}\n")));
        docs
    }
}

/// Generate the docstring for an `Op` variant.
fn op_docs(op: &Op) -> String {
    let arg_docs = bytecode_arg_docs(op.arg_bytes);
    let opcode_docs = format!("`0x{:02X}`\n\n", op.opcode);
    let desc = &op.description;
    let stack_in_docs = stack_in_docs(&op.stack_in);
    let stack_out_docs = stack_out_docs(&op.stack_out);
    let panic_docs = panic_docs(&op.panics);
    format!("{opcode_docs}\n{desc}\n{arg_docs}\n{stack_in_docs}\n{stack_out_docs}\n{panic_docs}")
}

/// Generate a single variant for an op group's enum decl.
fn op_enum_decl_variant(name: &str, node: &Node) -> syn::Variant {
    let ident = syn::Ident::new(name, Span::call_site());
    match node {
        Node::Group(group) => {
            let docs = &group.description;
            syn::parse_quote! {
                #[doc = #docs]
                #ident(#ident)
            }
        }
        Node::Op(op) => {
            let docs = op_docs(&op);
            match op.arg_bytes {
                0 => syn::parse_quote! {
                    #[doc = #docs]
                    #ident
                },
                8 => syn::parse_quote! {
                    #[doc = #docs]
                    #ident(essential_types::Word)
                },
                _ => panic!(
                    "Unexpected arg_bytes {}: requires more thoughtful asm-gen",
                    op.arg_bytes
                ),
            }
        }
    }
}

/// Generate a single variant for an op group's opcode enum decl
fn opcode_enum_decl_variant(name: &str, node: &Node) -> syn::Variant {
    let ident = syn::Ident::new(name, Span::call_site());
    match node {
        Node::Group(group) => {
            let docs = &group.description;
            syn::parse_quote! {
                #[doc = #docs]
                #ident(#ident)
            }
        }
        Node::Op(op) => {
            let docs = op_docs(&op);
            let opcode = op.opcode;
            syn::parse_quote! {
                #[doc = #docs]
                #ident = #opcode
            }
        }
    }
}

/// Generate the variants for an op group's enum decl.
fn op_enum_decl_variants(group: &Group) -> Punctuated<syn::Variant, Comma> {
    group
        .tree
        .iter()
        .map(|(name, node)| op_enum_decl_variant(name, node))
        .collect()
}

/// Generate the variants for an op group's opcode enum decl.
fn opcode_enum_decl_variants(group: &Group) -> Punctuated<syn::Variant, Comma> {
    group
        .tree
        .iter()
        .map(|(name, node)| opcode_enum_decl_variant(name, node))
        .collect()
}

/// Generate a single enum declaration from the given op group.
fn op_enum_decl(name: &str, group: &Group) -> syn::ItemEnum {
    let variants = op_enum_decl_variants(group);
    // Create the enum declaration for the group.
    let ident = syn::Ident::new(name, Span::call_site());
    let docs = &group.description;
    let item_enum = syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub enum #ident {
            #variants
        }
    };
    item_enum
}

/// Generate an opcode enum declaration for the given op group.
fn opcode_enum_decl(name: &str, group: &Group) -> syn::ItemEnum {
    let variants = opcode_enum_decl_variants(group);
    // When generating the opcode enum, the top-level type should be called `Opcode`.
    let name = if name == "Op" { "Opcode" } else { name };
    let ident = syn::Ident::new(name, Span::call_site());
    let docs = &group.description;
    let item_enum = syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[repr(u8)]
        pub enum #ident {
            #variants
        }
    };
    item_enum
}

/// Generate items related only to constraint execution.
fn constraint_items(tree: &Tree, new_item: impl Fn(&str, &Group) -> syn::Item) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::constraint_groups(tree, &mut |str, group| items.push(new_item(str, group)));
    items
}

/// Generate all op enum declarations for constraint execution.
fn constraint_op_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    constraint_items(tree, |name, group| {
        syn::Item::Enum(op_enum_decl(name, group))
    })
}

/// Generate all op enum declarations for constraint execution.
fn constraint_opcode_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    constraint_items(tree, |name, group| {
        syn::Item::Enum(opcode_enum_decl(name, group))
    })
}

/// Generate items related to state read execution, omitting those already
/// generated for constraint execution.
fn state_read_items(tree: &Tree, new_item: impl Fn(&str, &Group) -> syn::Item) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::state_read_groups(tree, &mut |str, group| items.push(new_item(str, group)));
    items
}

/// Generate all op enum declarations for state read execution besides those
/// already generated for constraint execution.
fn state_read_op_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    state_read_items(tree, |name, group| {
        syn::Item::Enum(op_enum_decl(name, group))
    })
}

/// Generate all opcode enum declarations for state read execution, omitting
/// those already generated for constraint execution.
fn state_read_opcode_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    state_read_items(tree, |name, group| {
        syn::Item::Enum(opcode_enum_decl(name, group))
    })
}

const DOCS_TABLE_HEADER: &str = "\n\n\
    | Opcode | Op | Short Description |\n\
    | --- | --- | --- |\n";

/// Generates a row for a single op within an ASM table docs.
fn docs_table_row(names: &[String], op: &Op) -> String {
    assert!(
        names.len() >= 2,
        "`names` should contain at least the group and op names"
    );
    let enum_ix = names.len() - 2;
    let enum_variant = &names[enum_ix..];
    let enum_name = enum_variant.first().unwrap();
    let variant_name = enum_variant.last().unwrap();
    let link = format!("enum.{enum_name}.html#variant.{variant_name}");
    let opcode_link = format!("./opcode/{link}");
    let op_link = format!("./op/{link}");
    let short_desc = op.description.lines().next().unwrap();
    format!(
        "| [`0x{:02X}`]({opcode_link}) | [{}]({op_link}) | {short_desc} |\n",
        op.opcode,
        enum_variant.join(" ")
    )
}

/// Generates a markdown table containing only the constraint operations.
fn constraint_ops_docs_table(tree: &Tree) -> syn::LitStr {
    let mut docs = DOCS_TABLE_HEADER.to_string();
    visit::constraint_ops(tree, &mut |names, op| {
        docs.push_str(&docs_table_row(names, op));
    });
    syn::parse_quote! { #docs }
}

/// Generates a markdown table containing all operations.
fn ops_docs_table(tree: &Tree) -> syn::LitStr {
    let mut docs = DOCS_TABLE_HEADER.to_string();
    visit::ops(tree, &mut |names, op| {
        docs.push_str(&docs_table_row(names, op));
    });
    syn::parse_quote! { #docs }
}

fn token_stream_from_items(items: impl IntoIterator<Item = syn::Item>) -> TokenStream {
    items
        .into_iter()
        .flat_map(|item| item.into_token_stream())
        .collect::<proc_macro2::TokenStream>()
        .into()
}

// ----------------------------------------------------------------------------

#[proc_macro]
pub fn gen_constraint_op_decls(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let items = constraint_op_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_opcode_decls(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let items = constraint_opcode_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_op_decls(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let items = state_read_op_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_opcode_decls(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let items = state_read_opcode_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_ops_docs_table(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let lit_str = constraint_ops_docs_table(&tree);
    lit_str.into_token_stream().into()
}

#[proc_macro]
pub fn gen_ops_docs_table(_input: TokenStream) -> TokenStream {
    let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let lit_str = ops_docs_table(&tree);
    lit_str.into_token_stream().into()
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Tree;

    #[test]
    fn test_op_tree() {
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        println!("{:#?}", tree);
    }

    #[test]
    fn test_no_duplicate_opcodes() {
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
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
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
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
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        let mut exists = false;
        super::visit::groups(&tree, &mut |name, _| exists |= name == CONSTRAINT_OP_NAME);
        assert!(
            exists,
            "The `Constraint` op is a special operation that is expected to exist and \
            is used to distinguish between constraint and state read ops. Hitting this \
            error implies the ASM yaml has been refactored or the Constraint name may \
            have changed. If so, the `CONSTRAINT_OP_NAME` should be updated.",
        );
    }
}
