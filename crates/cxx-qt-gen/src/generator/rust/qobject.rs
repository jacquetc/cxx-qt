// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use crate::{
    generator::{
        naming::{namespace::NamespaceName, qobject::QObjectName},
        rust::{
            constructor, cxxqttype, fragment::RustFragmentPair, inherit,
            method::generate_rust_methods, property::generate_rust_properties,
            signals::generate_rust_signals, threading,
        },
        utils::rust::syn_ident_cxx_bridge_to_qualified_impl,
    },
    parser::qobject::ParsedQObject,
};
use quote::quote;
use syn::{Ident, ImplItem, Item, Path, Result};

use super::qenum;

#[derive(Default)]
pub struct GeneratedRustQObject {
    /// Module for the CXX bridge
    pub cxx_mod_contents: Vec<Item>,
    /// Items for the CXX-Qt module
    pub cxx_qt_mod_contents: Vec<Item>,
}

impl GeneratedRustQObject {
    pub fn append(&mut self, other: &mut Self) {
        self.cxx_mod_contents.append(&mut other.cxx_mod_contents);
        self.cxx_qt_mod_contents
            .append(&mut other.cxx_qt_mod_contents);
    }

    pub fn from(
        qobject: &ParsedQObject,
        qualified_mappings: &BTreeMap<Ident, Path>,
        module_ident: &Ident,
    ) -> Result<GeneratedRustQObject> {
        // Create the base object
        let qobject_idents = QObjectName::from(qobject);
        let namespace_idents = NamespaceName::from(qobject);
        let mut generated = GeneratedRustQObject {
            cxx_mod_contents: vec![],
            cxx_qt_mod_contents: qobject.others.clone(),
        };

        generated.append(&mut generate_qobject_definitions(&qobject_idents)?);

        // Generate methods for the properties, invokables, signals
        generated.append(&mut generate_rust_properties(
            &qobject.properties,
            &qobject_idents,
            qualified_mappings,
        )?);
        generated.append(&mut generate_rust_methods(
            &qobject.methods,
            &qobject_idents,
        )?);
        generated.append(&mut generate_passthrough_impl(
            &qobject.passthrough_impl_items,
            &qobject_idents,
        )?);
        generated.append(&mut inherit::generate(
            &qobject_idents,
            &qobject.inherited_methods,
        )?);
        generated.append(&mut generate_rust_signals(
            &qobject.signals,
            &qobject_idents,
            qualified_mappings,
        )?);
        generated.append(&mut qenum::generate(&qobject.qenums));

        // If this type is a singleton then we need to add an include
        if let Some(qml_metadata) = &qobject.qml_metadata {
            if qml_metadata.singleton {
                let fragment = RustFragmentPair {
                    cxx_bridge: vec![quote! {
                        unsafe extern "C++" {
                            include!(<QtQml/QQmlEngine>);
                        }
                    }],
                    implementation: vec![],
                };
                generated
                    .cxx_mod_contents
                    .append(&mut fragment.cxx_bridge_as_items()?);
            }
        }

        // If this type has threading enabled then add generation
        if qobject.threading {
            generated.append(&mut threading::generate(
                &qobject_idents,
                &namespace_idents,
                qualified_mappings,
                module_ident,
            )?);
        }

        // If this type has locking enabling then implement the trait
        //
        // This could be implemented using an auto trait in the future once stable
        // https://doc.rust-lang.org/beta/unstable-book/language-features/auto-traits.html
        if qobject.locking {
            let qualified_impl = syn_ident_cxx_bridge_to_qualified_impl(
                &qobject_idents.cpp_class.rust,
                qualified_mappings,
            );
            generated.cxx_qt_mod_contents.push(syn::parse_quote! {
                impl cxx_qt::Locking for #qualified_impl {}
            });
        }

        generated.append(&mut constructor::generate(
            &qobject.constructors,
            &qobject_idents,
            &namespace_idents,
            qualified_mappings,
            module_ident,
        )?);

        generated.append(&mut cxxqttype::generate(
            &qobject_idents,
            qualified_mappings,
        )?);

        Ok(generated)
    }
}

/// Generate the non invokable methods for the Rust side
pub fn generate_passthrough_impl(
    impl_items: &[ImplItem],
    qobject_idents: &QObjectName,
) -> Result<GeneratedRustQObject> {
    let mut blocks = GeneratedRustQObject::default();
    blocks.cxx_qt_mod_contents.extend_from_slice(
        &impl_items
            .iter()
            // Build non invokables on the C++ struct
            .map(|impl_item| {
                let cpp_class_name_rust = &qobject_idents.cpp_class.rust;
                syn::parse2(quote! {
                    impl #cpp_class_name_rust {
                        #impl_item
                    }
                })
            })
            .collect::<Result<Vec<Item>>>()?,
    );

    Ok(blocks)
}

/// Generate the C++ and Rust CXX definitions for the QObject
fn generate_qobject_definitions(qobject_idents: &QObjectName) -> Result<GeneratedRustQObject> {
    let mut generated = GeneratedRustQObject::default();
    let cpp_class_name_rust = &qobject_idents.cpp_class.rust;
    let rust_struct_name_rust = &qobject_idents.rust_struct.rust;
    let rust_struct_name_rust_str = rust_struct_name_rust.to_string();
    let fragment = RustFragmentPair {
        cxx_bridge: vec![
            quote! {
                unsafe extern "C++" {
                    #[doc = "The C++ type for the QObject "]
                    #[doc = #rust_struct_name_rust_str]
                    #[doc = "\n"]
                    #[doc = "Use this type when referring to the QObject as a pointer"]
                    #[doc = "\n"]
                    #[doc = "See the book for more information: <https://kdab.github.io/cxx-qt/book/qobject/generated-qobject.html>"]
                    type #cpp_class_name_rust;
                }
            },
            quote! {
                extern "Rust" {
                    type #rust_struct_name_rust;
                }
            },
        ],
        implementation: vec![],
    };

    generated
        .cxx_mod_contents
        .append(&mut fragment.cxx_bridge_as_items()?);

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::Parser;
    use crate::tests::assert_tokens_eq;
    use quote::format_ident;
    use syn::{parse_quote, ItemMod};

    #[test]
    fn test_generated_rust_qobject_blocks_singleton() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    #[qml_element]
                    #[qml_singleton]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let rust = GeneratedRustQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &BTreeMap::<Ident, Path>::default(),
            &format_ident!("ffi"),
        )
        .unwrap();
        assert_eq!(rust.cxx_mod_contents.len(), 6);
        assert_tokens_eq(
            &rust.cxx_mod_contents[0],
            quote! {
                unsafe extern "C++" {
                    #[doc = "The C++ type for the QObject "]
                    #[doc = "MyObjectRust"]
                    #[doc = "\n"]
                    #[doc = "Use this type when referring to the QObject as a pointer"]
                    #[doc = "\n"]
                    #[doc = "See the book for more information: <https://kdab.github.io/cxx-qt/book/qobject/generated-qobject.html>"]
                    type MyObject;
                }
            },
        );
        assert_tokens_eq(
            &rust.cxx_mod_contents[1],
            quote! {
                extern "Rust" {
                    type MyObjectRust;
                }
            },
        );
        assert_tokens_eq(
            &rust.cxx_mod_contents[2],
            quote! {
                unsafe extern "C++" {
                    include!(<QtQml/QQmlEngine>);
                }
            },
        );
        assert_tokens_eq(
            &rust.cxx_mod_contents[3],
            quote! {
                extern "Rust" {
                    #[cxx_name = "createRs"]
                    #[namespace = "cxx_qt::cxx_qt_my_object"]
                    fn create_rs_my_object_rust() -> Box<MyObjectRust>;
                }
            },
        );
        assert_tokens_eq(
            &rust.cxx_mod_contents[4],
            quote! {
                unsafe extern "C++" {
                    #[cxx_name = "unsafeRust"]
                    #[doc(hidden)]
                    fn cxx_qt_ffi_rust(self: &MyObject) -> &MyObjectRust;
                }
            },
        );
        assert_tokens_eq(
            &rust.cxx_mod_contents[5],
            quote! {
                unsafe extern "C++" {
                    #[cxx_name = "unsafeRustMut"]
                    #[doc(hidden)]
                    fn cxx_qt_ffi_rust_mut(self: Pin<&mut MyObject>) -> Pin<&mut MyObjectRust>;
                }
            },
        );
    }
}
