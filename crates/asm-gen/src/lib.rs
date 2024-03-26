//! Generate essential ASM declarations from the official specification.
//!
//! Provides proc macros for generating declarations and implementations for
//! both the `essentail-constraint-asm` and `essential-state-asm` crates.

use essential_asm_spec::{visit, Group, Node, Op, StackOut, Tree};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{punctuated::Punctuated, token::Comma};

const WORD_SIZE: usize = std::mem::size_of::<essential_types::Word>();

/// Document the required bytecode arguments to the operation.
fn bytecode_arg_docs(num_arg_bytes: u8) -> String {
    if num_arg_bytes == 0 {
        return String::new();
    }
    assert_eq!(
        num_arg_bytes as usize % WORD_SIZE,
        0,
        "doc gen currently only supports arguments that are a multiple of the word size"
    );
    let arg_words = num_arg_bytes as usize / WORD_SIZE;
    format!(
        "## Bytecode Argument\nThis operation expects a {num_arg_bytes}-byte \
        ({arg_words}-word) argument following its opcode within bytecode."
    )
}

/// Generate an Op's stack-input docstring.
fn stack_in_docs(stack_in: &[String]) -> String {
    if stack_in.is_empty() {
        return String::new();
    }
    format!("## Stack Input\n`[{}]`\n", stack_in.join(", "))
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
        return String::new();
    }
    let mut docs = "## Panics\n".to_string();
    panic_reasons
        .iter()
        .for_each(|reason| docs.push_str(&format!("- {reason}\n")));
    docs
}

/// Generate the docstring for an `Op` variant.
fn op_docs(op: &Op) -> String {
    let arg_docs = bytecode_arg_docs(op.num_arg_bytes);
    let opcode_docs = format!("`0x{:02X}`\n\n", op.opcode);
    let desc = &op.description;
    let stack_in_docs = stack_in_docs(&op.stack_in);
    let stack_out_docs = stack_out_docs(&op.stack_out);
    let panic_docs = panic_docs(&op.panics);
    format!("{opcode_docs}\n{desc}\n{arg_docs}\n{stack_in_docs}\n{stack_out_docs}\n{panic_docs}")
}

/// Generate the docstring for an `Opcode` variant.
fn opcode_docs(enum_name: &str, name: &str, op: &Op) -> String {
    let opcode_docs = format!("`0x{:02X}`\n\n", op.opcode);
    let docs = format!(
        "Opcode associated with the \
        [{enum_name}::{name}][super::op::{enum_name}::{name}] operation."
    );
    format!("{opcode_docs}\n{docs}")
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
            let docs = op_docs(op);
            match op.num_arg_bytes {
                0 => syn::parse_quote! {
                    #[doc = #docs]
                    #ident
                },
                8 => syn::parse_quote! {
                    #[doc = #docs]
                    #ident(essential_types::Word)
                },
                _ => panic!(
                    "Unexpected num_arg_bytes {}: requires more thoughtful asm-gen",
                    op.num_arg_bytes
                ),
            }
        }
    }
}

/// Generate a single variant for an op group's opcode enum decl
fn opcode_enum_decl_variant(parent_name: &str, name: &str, node: &Node) -> syn::Variant {
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
            let docs = opcode_docs(parent_name, name, op);
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
fn opcode_enum_decl_variants(enum_name: &str, group: &Group) -> Punctuated<syn::Variant, Comma> {
    group
        .tree
        .iter()
        .map(|(name, node)| opcode_enum_decl_variant(enum_name, name, node))
        .collect()
}

/// Generate a single enum declaration from the given op group.
fn op_enum_decl(name: &str, group: &Group) -> syn::ItemEnum {
    let variants = op_enum_decl_variants(group);
    let ident = syn::Ident::new(name, Span::call_site());
    let docs = &group.description;
    let item_enum = syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum #ident {
            #variants
        }
    };
    item_enum
}

/// Generate an opcode enum declaration for the given op group.
fn opcode_enum_decl(name: &str, group: &Group) -> syn::ItemEnum {
    let variants = opcode_enum_decl_variants(name, group);
    let ident = syn::Ident::new(name, Span::call_site());
    let docs = &group.description;
    let item_enum = syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(u8)]
        pub enum #ident {
            #variants
        }
    };
    item_enum
}

/// Generate the arms of the `opcode` method's match expression.
fn op_enum_impl_opcode_arms(enum_ident: &syn::Ident, group: &Group) -> Vec<syn::Arm> {
    group
        .tree
        .iter()
        .map(|(name, node)| {
            let name = syn::Ident::new(name, Span::call_site());
            match node {
                Node::Group(_) => syn::parse_quote! {
                    #enum_ident::#name(group) => crate::opcode::#enum_ident::#name(group.opcode()),
                },
                Node::Op(op) if op.num_arg_bytes > 0 => {
                    syn::parse_quote! {
                        #enum_ident::#name(_) => crate::opcode::#enum_ident::#name,
                    }
                }
                Node::Op(_) => {
                    syn::parse_quote! {
                        #enum_ident::#name => crate::opcode::#enum_ident::#name,
                    }
                }
            }
        })
        .collect()
}

/// Generate the `opcode` method implementation for an operation type.
fn op_enum_impl_opcode(name: &str, group: &Group) -> syn::ItemImpl {
    let name = syn::Ident::new(name, Span::call_site());
    let arms = op_enum_impl_opcode_arms(&name, group);
    syn::parse_quote! {
        impl #name {
            /// The opcode associated with the operation.
            pub fn opcode(&self) -> crate::opcode::#name {
                match *self {
                    #(
                        #arms
                    )*
                }
            }
        }
    }
}

/// Generate a single variant for an op enum's bytes iterator.
fn op_enum_bytes_iter_decl_variant(name: &str, node: &Node) -> syn::Variant {
    let ident = syn::Ident::new(name, Span::call_site());
    match node {
        Node::Group(_group) => {
            syn::parse_quote! {
                #ident(#ident)
            }
        }
        Node::Op(op) => {
            // The opcode + the number of bytes in the associated data.
            let n_bytes: usize = 1 + op.num_arg_bytes as usize;
            let n_bytes: syn::LitInt = syn::parse_quote!(#n_bytes);
            syn::parse_quote! {
                #ident {
                    /// The current index within the bytes array.
                    index: usize,
                    /// The operation's associated data as an array of bytes.
                    bytes: [u8; #n_bytes],
                }
            }
        }
    }
}

/// Generate an op enum's bytes iterator variants.
fn op_enum_bytes_iter_decl_variants(group: &Group) -> Vec<syn::Variant> {
    group
        .tree
        .iter()
        .map(|(name, node)| op_enum_bytes_iter_decl_variant(name, node))
        .collect()
}

/// Create the declaration for an Op enum's associated bytes iterator type.
fn op_enum_bytes_iter_decl(name: &str, group: &Group) -> syn::ItemEnum {
    let name = syn::Ident::new(name, Span::call_site());
    let variants = op_enum_bytes_iter_decl_variants(group);
    let docs = format!(
        "The bytes iterator produced by the \
        [{name}::to_bytes][super::{name}::to_bytes] method."
    );
    syn::parse_quote! {
        #[doc = #docs]
        #[derive(Clone, Debug)]
        pub enum #name {
            #(
                /// Bytes iterator for the op variant.
                #variants
            ),*
        }
    }
}

/// An arm of the match expr within a bytes iterator implementation.
fn op_enum_bytes_iter_impl_arm(name: &str, node: &Node) -> syn::Arm {
    let name = syn::Ident::new(name, Span::call_site());
    match node {
        Node::Group(_group) => syn::parse_quote! {
            Self::#name(ref mut iter) => iter.next(),
        },
        Node::Op(_op) => {
            syn::parse_quote! {
                Self::#name { ref mut index, ref bytes } => {
                    let byte = *bytes.get(*index)?;
                    *index += 1;
                    Some(byte)
                },
            }
        }
    }
}

/// The arms of the match expr within a bytes iterator implementation.
fn op_enum_bytes_iter_impl_arms(group: &Group) -> Vec<syn::Arm> {
    group
        .tree
        .iter()
        .map(|(name, node)| op_enum_bytes_iter_impl_arm(name, node))
        .collect()
}

/// An implementation of Iterator for the op's associated bytes iterator type.
fn op_enum_bytes_iter_impl(name: &str, group: &Group) -> syn::ItemImpl {
    let bytes_iter = syn::Ident::new(name, Span::call_site());
    let arms = op_enum_bytes_iter_impl_arms(group);
    syn::parse_quote! {
        impl Iterator for #bytes_iter {
            type Item = u8;
            fn next(&mut self) -> Option<Self::Item> {
                match *self {
                    #(
                        #arms
                    )*
                }
            }
        }
    }
}

/// Generate an arm of the match expr with an op type's `to_bytes` method.
fn op_enum_impl_to_bytes_arm(enum_name: &syn::Ident, name: &str, node: &Node) -> syn::Arm {
    let name = syn::Ident::new(name, Span::call_site());
    match node {
        Node::Group(_group) => {
            syn::parse_quote! {
                Self::#name(group) => bytes_iter::#enum_name::#name(group.to_bytes()),
            }
        }
        Node::Op(op) => {
            let opcode = op.opcode;
            if op.num_arg_bytes == 0 {
                syn::parse_quote! {
                    Self::#name => bytes_iter::#enum_name::#name {
                        index: 0usize,
                        bytes: [#opcode],
                    },
                }
            } else if op.num_arg_bytes == 8 {
                syn::parse_quote! {
                    Self::#name(data) => {
                        use essential_types::convert::bytes_from_word;
                        let [b0, b1, b2, b3, b4, b5, b6, b7] = bytes_from_word(data.clone());
                        bytes_iter::#enum_name::#name {
                            index: 0usize,
                            bytes: [#opcode, b0, b1, b2, b3, b4, b5, b6, b7],
                        }
                    },
                }
            } else {
                panic!(
                    "Currently only support operations with a single word \
                    argument. This must be updated to properly support variable \
                    size arguments.",
                )
            }
        }
    }
}

/// Generate the arms of the match expr with an op type's `to_bytes` method.
fn op_enum_impl_to_bytes_arms(enum_name: &syn::Ident, group: &Group) -> Vec<syn::Arm> {
    group
        .tree
        .iter()
        .map(|(name, node)| op_enum_impl_to_bytes_arm(enum_name, name, node))
        .collect()
}

/// Generate the `to_bytes` method for an operation type.
fn op_enum_impl_to_bytes(name: &str, group: &Group) -> syn::ItemImpl {
    let name = syn::Ident::new(name, Span::call_site());
    let arms = op_enum_impl_to_bytes_arms(&name, group);
    syn::parse_quote! {
        impl #name {
            /// Convert the operation to its serialized form in bytes.
            pub fn to_bytes(&self) -> bytes_iter::#name {
                match self {
                    #(
                        #arms
                    )*
                }
            }
        }
    }
}

/// Generate a `From` implementation for converting the subgroup (last name) to
/// the higher-level group (first name).
fn impl_from_subgroup(names: &[String]) -> syn::ItemImpl {
    let ident = syn::Ident::new(names.first().unwrap(), Span::call_site());
    let subident = syn::Ident::new(names.last().unwrap(), Span::call_site());
    let inner_expr: syn::Expr = syn::parse_quote!(subgroup);
    let expr = enum_variant_tuple1_expr(names, inner_expr);
    syn::parse_quote! {
        impl From<#subident> for #ident {
            fn from(subgroup: #subident) -> Self {
                #expr
            }
        }
    }
}

/// Generate the `From` implementations for converting subgroups to higher-level groups.
fn impl_from_subgroups(name: &str, group: &Group) -> Vec<syn::ItemImpl> {
    let mut impls = vec![];
    let mut names = vec![name.to_string()];
    visit::groups_filtered_recurse(&group.tree, &|_| true, &mut names, &mut |names, _group| {
        impls.push(impl_from_subgroup(names));
    });
    impls
}

/// Wrap an expression with the given nested op group naming.
/// E.g. the args [StateRead, Constraint, Stack]` and `my_expr` becomes
/// `StateRead::Constraint(Constraint::Stack(my_expr))`.
fn enum_variant_tuple1_expr(names: &[String], mut expr: syn::Expr) -> syn::Expr {
    assert!(!names.is_empty(), "Expecting at least one variant name");
    let mut idents: Vec<_> = names
        .iter()
        .map(|n| syn::Ident::new(n, Span::call_site()))
        .collect();
    let mut variant_name = idents.pop().unwrap();
    while let Some(enum_name) = idents.pop() {
        expr = syn::parse_quote!(#enum_name::#variant_name(#expr));
        variant_name = enum_name;
    }
    expr
}

/// Generate an opcode expression from the given nested op group naming.
/// E.g. `[Constraint, Stack, Push]` becomes `Constraint::Stack(Stack::Push)`.
fn opcode_expr_from_names(names: &[String]) -> syn::Expr {
    assert!(
        names.len() >= 2,
        "Expecting at least the enum and variant names"
    );
    let enum_name = syn::Ident::new(&names[names.len() - 2], Span::call_site());
    let variant_name = syn::Ident::new(&names[names.len() - 1], Span::call_site());
    let expr = syn::parse_quote!(#enum_name::#variant_name);
    enum_variant_tuple1_expr(&names[..names.len() - 1], expr)
}

/// Generates an arm of the match expr used within the opcode's `parse_op` implementation.
fn opcode_enum_impl_parse_op_arm(
    enum_name: &syn::Ident,
    name: &syn::Ident,
    node: &Node,
) -> syn::Arm {
    match node {
        Node::Group(_group) => {
            syn::parse_quote! {
                Self::#name(group) => group.parse_op(bytes).map(Into::into),
            }
        }
        Node::Op(op) if op.num_arg_bytes == 0 => {
            syn::parse_quote! {
                Self::#name => Ok(crate::op::#enum_name::#name),
            }
        }
        // TODO: Update this to handle variable size arguments if we add more
        // sophisticated options than `Push`.
        Node::Op(op) => {
            assert_eq!(
                op.num_arg_bytes as usize % WORD_SIZE,
                0,
                "Currently only support operations with an `num_arg_bytes` that is \
                a multiple of the word size",
            );
            let words = op.num_arg_bytes as usize / WORD_SIZE;
            assert_eq!(
                words, 1,
                "Currently only support operations with a single word \
                argument. This must be updated to properly support variable \
                size arguments.",
            );
            syn::parse_quote! {
                Self::#name => {
                    use essential_types::convert::word_from_bytes;
                    fn parse_word_bytes(bytes: &mut impl Iterator<Item = u8>) -> Option<[u8; 8]> {
                        Some([
                            bytes.next()?, bytes.next()?, bytes.next()?, bytes.next()?,
                            bytes.next()?, bytes.next()?, bytes.next()?, bytes.next()?,
                        ])
                    }
                    let word_bytes: [u8; 8] = parse_word_bytes(bytes).ok_or(NotEnoughBytesError)?;
                    let word: essential_types::Word = word_from_bytes(word_bytes);
                    Ok(crate::op::#enum_name::#name(word))
                },
            }
        }
    }
}

/// Generates the arms of the match expr used within the opcode's `parse_op` implementation.
fn opcode_enum_impl_parse_op_arms(enum_name: &syn::Ident, group: &Group) -> Vec<syn::Arm> {
    group
        .tree
        .iter()
        .map(|(name, node)| {
            let name = syn::Ident::new(name, Span::call_site());
            opcode_enum_impl_parse_op_arm(enum_name, &name, node)
        })
        .collect()
}

/// Generate a method that parses the operation associated with the opcode.
fn opcode_enum_impl_parse_op(name: &str, group: &Group) -> syn::ItemImpl {
    let ident = syn::Ident::new(name, Span::call_site());
    let arms = opcode_enum_impl_parse_op_arms(&ident, group);
    syn::parse_quote! {
        impl #ident {
            /// Attempt to parse the operation associated with the opcode from the given bytes.
            ///
            /// Only consumes the bytes necessary to construct any associated data.
            ///
            /// Returns an error in the case that the given `bytes` iterator
            /// contains insufficient bytes to parse the op.
            pub fn parse_op(
                &self,
                bytes: &mut impl Iterator<Item = u8>,
            ) -> Result<crate::op::#ident, NotEnoughBytesError> {
                match *self {
                    #(
                        #arms
                    )*
                }
            }
        }
    }
}

/// Generate the arms from the opcode's `TryFrom<u8>` conversion match expr.
fn opcode_enum_impl_tryfrom_u8_arms(group: &Group) -> Vec<syn::Arm> {
    let mut arms = vec![];
    let mut names = vec!["Self".to_string()];
    visit::ops_filtered_recurse(&group.tree, &|_| true, &mut names, &mut |names, op| {
        let opcode = op.opcode;
        let opcode_expr = opcode_expr_from_names(names);
        let arm = syn::parse_quote! {
            #opcode => #opcode_expr,
        };
        arms.push(arm);
    });
    arms
}

/// Generate the arms for the conversion from opcode to u8.
fn opcode_enum_impl_from_opcode_for_u8_arms(
    enum_ident: &syn::Ident,
    group: &Group,
) -> Vec<syn::Arm> {
    group
        .tree
        .iter()
        .map(|(name, node)| {
            let name = syn::Ident::new(name, Span::call_site());
            match node {
                Node::Group(_) => syn::parse_quote! {
                    #enum_ident::#name(group) => u8::from(group),
                },
                Node::Op(op) => {
                    let opcode = op.opcode;
                    syn::parse_quote! {
                        #enum_ident::#name => #opcode,
                    }
                }
            }
        })
        .collect()
}

/// Generate the conversion from the opcode type to `u8`.
fn opcode_enum_impl_from_opcode_for_u8(name: &str, group: &Group) -> syn::ItemImpl {
    let name = syn::Ident::new(name, Span::call_site());
    let arms = opcode_enum_impl_from_opcode_for_u8_arms(&name, group);
    syn::parse_quote! {
        impl From<#name> for u8 {
            fn from(opcode: #name) -> Self {
                match opcode {
                    #(
                        #arms
                    )*
                }
            }
        }
    }
}

/// Generate the fallible conversion from `u8` to the opcode.
fn opcode_enum_impl_tryfrom_u8(name: &str, group: &Group) -> syn::ItemImpl {
    let name = syn::Ident::new(name, Span::call_site());
    let arms = opcode_enum_impl_tryfrom_u8_arms(group);
    syn::parse_quote! {
        impl TryFrom<u8> for #name {
            type Error = InvalidOpcodeError;
            fn try_from(u: u8) -> Result<Self, Self::Error> {
                let opcode = match u {
                    #(
                        #arms
                    )*
                    _ => return Err(InvalidOpcodeError(u)),
                };
                Ok(opcode)
            }
        }
    }
}

/// Generate the implementations for the given op group enum.
fn op_enum_impls(names: &[String], group: &Group) -> Vec<syn::ItemImpl> {
    let name = names.last().unwrap();
    let mut impls = vec![
        op_enum_impl_opcode(name, group),
        op_enum_impl_to_bytes(name, group),
    ];
    impls.extend(impl_from_subgroups(name, group));
    impls
}

/// Generate the implementation for the opcode enum.
fn opcode_enum_impls(names: &[String], group: &Group) -> Vec<syn::ItemImpl> {
    let name = names.last().unwrap();
    let mut impls = vec![
        opcode_enum_impl_from_opcode_for_u8(name, group),
        opcode_enum_impl_tryfrom_u8(name, group),
        opcode_enum_impl_parse_op(name, group),
    ];
    impls.extend(impl_from_subgroups(name, group));
    impls
}

/// Generate items related only to constraint execution.
fn constraint_items(
    tree: &Tree,
    new_item: impl Fn(&[String], &Group) -> syn::Item,
) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::constraint_groups(tree, &mut |str, group| items.push(new_item(str, group)));
    items
}

/// Generate all op enum declarations for constraint execution.
fn constraint_op_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    constraint_items(tree, |names, group| {
        let name = names.last().unwrap();
        syn::Item::Enum(op_enum_decl(name, group))
    })
}

/// Generate all opcode enum declarations for constraint execution.
fn constraint_opcode_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    constraint_items(tree, |names, group| {
        let name = names.last().unwrap();
        syn::Item::Enum(opcode_enum_decl(name, group))
    })
}

/// Generate the bytes iterator declaration and implementation for all op groups.
fn constraint_op_enum_bytes_iter(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::constraint_groups(tree, &mut |names, group| {
        let name = names.last().unwrap();
        items.push(syn::Item::Enum(op_enum_bytes_iter_decl(name, group)));
        items.push(syn::Item::Impl(op_enum_bytes_iter_impl(name, group)));
    });
    items
}

/// Generate all op enum implementations for constraint execution.
fn constraint_op_enum_impls(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::constraint_groups(tree, &mut |names, group| {
        items.extend(op_enum_impls(names, group).into_iter().map(syn::Item::Impl));
    });
    items
}

/// Generate all opcode enum implementations for constraint execution.
fn constraint_opcode_enum_impls(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::constraint_groups(tree, &mut |name, group| {
        items.extend(
            opcode_enum_impls(name, group)
                .into_iter()
                .map(syn::Item::Impl),
        );
    });
    items
}

/// Generate items related to state read execution, omitting those already
/// generated for constraint execution.
fn state_read_items(
    tree: &Tree,
    new_item: impl Fn(&[String], &Group) -> syn::Item,
) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::state_read_groups(tree, &mut |str, group| items.push(new_item(str, group)));
    items
}

/// Generate all op enum declarations for state read execution besides those
/// already generated for constraint execution.
fn state_read_op_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    state_read_items(tree, |names, group| {
        let name = names.last().unwrap();
        syn::Item::Enum(op_enum_decl(name, group))
    })
}

/// Generate all opcode enum declarations for state read execution, omitting
/// those already generated for constraint execution.
fn state_read_opcode_enum_decls(tree: &Tree) -> Vec<syn::Item> {
    state_read_items(tree, |names, group| {
        let name = names.last().unwrap();
        syn::Item::Enum(opcode_enum_decl(name, group))
    })
}

/// Generate the bytes iterator declaration and implementation for all op groups.
fn state_read_op_enum_bytes_iter(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::state_read_groups(tree, &mut |names, group| {
        let name = names.last().unwrap();
        items.push(syn::Item::Enum(op_enum_bytes_iter_decl(name, group)));
        items.push(syn::Item::Impl(op_enum_bytes_iter_impl(name, group)));
    });
    items
}

/// Generate all opcode enum implementations for constraint execution.
fn state_read_op_enum_impls(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::state_read_groups(tree, &mut |name, group| {
        items.extend(op_enum_impls(name, group).into_iter().map(syn::Item::Impl));
    });
    items
}

/// Generate all opcode enum implementations for constraint execution.
fn state_read_opcode_enum_impls(tree: &Tree) -> Vec<syn::Item> {
    let mut items = vec![];
    visit::state_read_groups(tree, &mut |name, group| {
        items.extend(
            opcode_enum_impls(name, group)
                .into_iter()
                .map(syn::Item::Impl),
        );
    });
    items
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
    let opcode_link = format!("opcode::{enum_name}::{variant_name}");
    let op_link = format!("op::{enum_name}::{variant_name}");
    let short_desc = op.description.lines().next().unwrap();
    format!(
        "| [`0x{:02X}`][{opcode_link}] | [{}][{op_link}] | {short_desc} |\n",
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

#[proc_macro]
pub fn gen_constraint_op_decls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = constraint_op_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_opcode_decls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = constraint_opcode_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_op_bytes_iter(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = constraint_op_enum_bytes_iter(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_op_impls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = constraint_op_enum_impls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_opcode_impls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = constraint_opcode_enum_impls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_op_decls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = state_read_op_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_opcode_decls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = state_read_opcode_enum_decls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_op_bytes_iter(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = state_read_op_enum_bytes_iter(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_op_impls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = state_read_op_enum_impls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_state_read_opcode_impls(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let items = state_read_opcode_enum_impls(&tree);
    token_stream_from_items(items)
}

#[proc_macro]
pub fn gen_constraint_ops_docs_table(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let lit_str = constraint_ops_docs_table(&tree);
    lit_str.into_token_stream().into()
}

#[proc_macro]
pub fn gen_ops_docs_table(_input: TokenStream) -> TokenStream {
    let tree = essential_asm_spec::tree();
    let lit_str = ops_docs_table(&tree);
    lit_str.into_token_stream().into()
}
