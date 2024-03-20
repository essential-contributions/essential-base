use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use serde::Deserialize;
use syn::{punctuated::Punctuated, token::Comma};

mod de;

const ASM_YAML: &str = include_str!("./../asm.yml");

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

/// Recursively visit all group nodes within the op tree in depth-first visit order.
fn visit_groups(tree: &Tree, f: &mut impl FnMut(&str, &Group)) {
    for (name, node) in tree.iter() {
        match node {
            Node::Group(g) => {
                f(name, g);
                visit_groups(&g.tree, f);
            }
            _ => (),
        }
    }
}

/// Recursively visit all operations in order of their opcode, where the first argument to
/// the given function provides the fully nested name.
fn visit_ops(tree: &Tree, f: &mut impl FnMut(&[String], &Op)) {
    fn visit_ops_inner(tree: &Tree, names: &mut Vec<String>, f: &mut impl FnMut(&[String], &Op)) {
        for (name, node) in tree.iter() {
            names.push(name.to_string());
            match node {
                Node::Group(g) => visit_ops_inner(&g.tree, names, f),
                Node::Op(op) => f(names, op),
            }
            names.pop();
        }
    }
    visit_ops_inner(tree, &mut vec![], f)
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

/// Generate an Op's stack-in docstring.
fn stack_in_docs(stack_in: &[String]) -> String {
    if stack_in.is_empty() {
        String::new()
    } else {
        format!("## Stack Input\n`[{}]`\n", stack_in.join(", "))
    }
}

/// Generate an Op's stack-out docstring.
fn stack_out_docs(stack_out: &StackOut) -> String {
    match stack_out {
        StackOut::Fixed(words) if words.is_empty() => String::new(),
        StackOut::Fixed(words) => {
            format!("## Stack Output\n`[{}]`\n", words.join(", "))
        }
        StackOut::Dynamic(out) => {
            format!(
                "## Stack Output\nThe stack output length depends on the \
                value of the `{}` stack input word.\n",
                out.len
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

/// Generate all op enum declarations from the top-level operation tree.
fn op_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    let mut enums = vec![];
    visit_groups(tree, &mut |name, group| {
        enums.push(op_enum_decl(name, group))
    });
    enums.into_iter().map(syn::Item::Enum).collect()
}

/// Generate all opcode enum declarations from the top-level op tree.
fn opcode_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    let mut enums = vec![];
    visit_groups(tree, &mut |name, group| {
        enums.push(opcode_enum_decl(name, group))
    });
    enums.into_iter().map(syn::Item::Enum).collect()
}

/// Generate the `op` module, containing all operations.
fn op_mod(tree: &Tree) -> syn::ItemMod {
    let enum_decls = op_enum_decls(tree);
    syn::parse_quote! {
        pub mod op {
            #(
                #enum_decls
            )*
        }
    }
}

/// Generate the `op` module, containing all operations.
fn opcode_mod(tree: &Tree) -> syn::ItemMod {
    let enum_decls = opcode_enum_decls(tree);
    syn::parse_quote! {
        pub mod opcode {
            #(
                #enum_decls
            )*
        }
    }
}

/// Produce the crate-root documentation.
fn asm_table_docs_from_op_tree(op_tree: &Tree) -> syn::LitStr {
    let mut docs = "\n\n| Opcode | Op | Short Description |\n| --- | --- | --- |\n".to_string();
    visit_ops(op_tree, &mut |names, op| {
        let enum_ix = names.len() - 2;
        let enum_variant = &names[enum_ix..];
        let enum_name = enum_variant.first().unwrap();
        let variant_name = enum_variant.last().unwrap();
        let link = format!("enum.{enum_name}.html#variant.{variant_name}");
        let opcode_link = format!("./opcode/{link}");
        let op_link = format!("./op/{link}");
        let short_desc = op.description.lines().next().unwrap();
        let line = format!(
            "| [`0x{:02X}`]({opcode_link}) | [{}]({op_link}) | {short_desc} |\n",
            op.opcode,
            enum_variant.join(" ")
        );
        docs.push_str(&line);
    });
    syn::parse_quote! { #docs }
}

#[proc_macro]
pub fn asm_gen(_input: TokenStream) -> TokenStream {
    let op_tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let op_mod = op_mod(&op_tree);
    let opcode_mod = opcode_mod(&op_tree);
    let mut stream = proc_macro2::TokenStream::default();
    stream.extend(op_mod.into_token_stream());
    stream.extend(opcode_mod.into_token_stream());
    stream.into()
}

#[proc_macro]
pub fn asm_table_docs(_input: TokenStream) -> TokenStream {
    let op_tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
    let doc_attr = asm_table_docs_from_op_tree(&op_tree);
    doc_attr.into_token_stream().into()
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::Tree;
    use quote::ToTokens;

    #[test]
    fn test_op_tree() {
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        println!("{:#?}", tree);
    }

    #[test]
    fn test_no_duplicate_opcodes() {
        let tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        let mut opcodes = std::collections::BTreeSet::new();
        super::visit_ops(&tree, &mut |name, op| {
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
    fn test_op_enum_decls() {
        let op_tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        let enum_decls = super::op_enum_decls(&op_tree);
        for decl in enum_decls {
            println!("\n{}", decl.to_token_stream());
        }
    }
}
