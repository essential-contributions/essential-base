use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use serde::Deserialize;

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
    shorthand: Option<String>,
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

// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------

/// Visit all group nodes within the op tree in depth-first visit order.
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

/// Visit all operations in order of their opcode, where the first argument to
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

/// A single enum declaration from an op group.
fn enum_decl_from_group(name: &str, group: &Group) -> syn::ItemEnum {
    use syn::{punctuated::Punctuated, token::Comma};
    // Collect variant AST nodes from the group's immediate children.
    let variants: Punctuated<syn::Variant, Comma> = group
        .tree
        .iter()
        .map(|(name, node)| {
            let ident = syn::Ident::new(name, Span::call_site());
            let variant: syn::Variant = match node {
                Node::Group(group) => {
                    let docs = &group.description;
                    syn::parse_quote! {
                        #[doc = #docs]
                        #ident(#ident)
                    }
                }
                Node::Op(op) => {
                    let opcode_docs = format!("`0x{:02X}`\n\n", op.opcode);
                    let docs = &op.description;

                    let stack_in_docs = if op.stack_in.is_empty() {
                        String::new()
                    } else {
                        format!("## Stack Input\n`[{}]`\n", op.stack_in.join(", "))
                    };

                    let stack_out_docs = match &op.stack_out {
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
                    };

                    let panic_docs = if op.panics.is_empty() {
                        String::new()
                    } else {
                        let mut docs = "## Panics\n".to_string();
                        op.panics
                            .iter()
                            .for_each(|reason| docs.push_str(&format!("- {reason}\n")));
                        docs
                    };

                    match op.arg_bytes {
                        0 => syn::parse_quote! {
                            #[doc = #opcode_docs]
                            #[doc = #docs]
                            #[doc = #stack_in_docs]
                            #[doc = #stack_out_docs]
                            #[doc = #panic_docs]
                            #ident
                        },
                        8 => syn::parse_quote! {
                            #[doc = #opcode_docs]
                            #[doc = #docs]
                            #[doc = #stack_in_docs]
                            #[doc = #stack_out_docs]
                            #[doc = #panic_docs]
                            #ident(essential_types::Word)
                        },
                        _ => panic!(
                            "Unexpected arg_bytes {}: requires more thoughtful asm-gen",
                            op.arg_bytes
                        ),
                    }
                }
            };
            variant
        })
        .collect();

    // Create the enum declaration for the group.
    let ident = syn::Ident::new(name, Span::call_site());
    let docs = &group.description;
    let item_enum = syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        // #[repr(u8)]
        pub enum #ident {
            #variants
        }
    };
    item_enum
}

/// Generate all enum declarations from the top-level operation tree.
fn enum_decls_from_op_tree(op_tree: &Tree) -> Vec<syn::Item> {
    let mut enums = vec![];
    visit_groups(op_tree, &mut |name, group| {
        enums.push(enum_decl_from_group(name, group))
    });
    enums.into_iter().map(syn::Item::Enum).collect()
}

/// Produce the crate-root documentation.
fn asm_table_docs_from_op_tree(op_tree: &Tree) -> syn::LitStr {
    let mut docs =
        "\n\n| Opcode | Op | Shorthand | Short Description |\n| --- | --- | --- | --- |\n"
            .to_string();
    visit_ops(op_tree, &mut |names, op| {
        let enum_ix = names.len() - 2;
        let enum_variant = &names[enum_ix..];
        let enum_name = enum_variant.first().unwrap();
        let variant_name = enum_variant.last().unwrap();
        let link = format!("./enum.{enum_name}.html#variant.{variant_name}");
        let shorthand = op
            .shorthand
            .clone()
            .unwrap_or_else(|| variant_name.to_uppercase());
        let short_desc = op.description.lines().next().unwrap();
        let line = format!(
            "| `0x{:02X}` | [{}]({link}) | `{shorthand}` | {short_desc} |\n",
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
    let enum_decls = enum_decls_from_op_tree(&op_tree);
    let mut stream = proc_macro2::TokenStream::default();
    for decl in enum_decls {
        stream.extend(decl.into_token_stream());
    }
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
    fn test_enum_decls() {
        let op_tree = serde_yaml::from_str::<Tree>(crate::ASM_YAML).unwrap();
        let enum_decls = super::enum_decls_from_op_tree(&op_tree);
        for decl in enum_decls {
            println!("\n{}", decl.to_token_stream());
        }
    }
}
