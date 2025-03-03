// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::generator::{
    cpp::{
        constructor, cxxqttype, fragment::CppFragment, inherit, locking,
        method::generate_cpp_methods, property::generate_cpp_properties, qenum,
        signal::generate_cpp_signals, threading,
    },
    naming::{namespace::NamespaceName, qobject::QObjectName},
};
use crate::parser::{mappings::ParsedCxxMappings, qobject::ParsedQObject};
use std::collections::BTreeSet;
use syn::Result;

#[derive(Default)]
pub struct GeneratedCppQObjectBlocks {
    /// List of forward declares before the class and include of the generated CXX header
    pub forward_declares: Vec<String>,
    /// List of Qt Meta Object items (eg Q_PROPERTY)
    pub metaobjects: Vec<String>,
    /// List of public methods for the QObject
    pub methods: Vec<CppFragment>,
    /// List of private methods for the QObject
    pub private_methods: Vec<CppFragment>,
    /// List of includes
    pub includes: BTreeSet<String>,
    /// Base class of the QObject
    pub base_classes: Vec<String>,
}

impl GeneratedCppQObjectBlocks {
    pub fn append(&mut self, other: &mut Self) {
        self.forward_declares.append(&mut other.forward_declares);
        self.metaobjects.append(&mut other.metaobjects);
        self.methods.append(&mut other.methods);
        self.private_methods.append(&mut other.private_methods);
        self.includes.append(&mut other.includes);
        self.base_classes.append(&mut other.base_classes);
    }

    pub fn from(qobject: &ParsedQObject) -> GeneratedCppQObjectBlocks {
        let mut qml_specifiers = Vec::new();
        if let Some(qml_metadata) = &qobject.qml_metadata {
            // Somehow moc doesn't include the info in metatypes.json that qmltyperegistrar needs
            // when using the QML_ELEMENT/QML_NAMED_ELEMENT macros, but moc works when using what
            // those macros expand to.
            qml_specifiers.push(format!(
                "Q_CLASSINFO(\"QML.Element\", \"{}\")",
                qml_metadata.name
            ));

            if qml_metadata.uncreatable {
                qml_specifiers.push("Q_CLASSINFO(\"QML.Creatable\", \"false\")".to_owned());
            }

            if qml_metadata.singleton {
                qml_specifiers.push("QML_SINGLETON".to_owned());
            }
        }
        GeneratedCppQObjectBlocks {
            metaobjects: qml_specifiers,
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct GeneratedCppQObject {
    /// Ident of the C++ QObject
    pub ident: String,
    /// Ident of the Rust object
    pub rust_ident: String,
    /// Ident of the namespace for CXX-Qt internals of the QObject
    pub namespace_internals: String,
    /// The blocks of the QObject
    pub blocks: GeneratedCppQObjectBlocks,
}

impl GeneratedCppQObject {
    pub fn from(
        qobject: &ParsedQObject,
        cxx_mappings: &ParsedCxxMappings,
    ) -> Result<GeneratedCppQObject> {
        // Create the base object
        let qobject_idents = QObjectName::from(qobject);
        let namespace_idents = NamespaceName::from(qobject);
        let cpp_class = qobject_idents.cpp_class.cpp.to_string();
        let mut generated = GeneratedCppQObject {
            ident: cpp_class.clone(),
            rust_ident: qobject_idents.rust_struct.cpp.to_string(),
            namespace_internals: namespace_idents.internal,
            blocks: GeneratedCppQObjectBlocks::from(qobject),
        };

        // Ensure that we include MaybeLockGuard<T> that is used in multiple places
        generated
            .blocks
            .includes
            .insert("#include <cxx-qt-common/cxxqt_maybelockguard.h>".to_owned());

        // Build the base class
        let base_class = qobject
            .base_class
            .clone()
            .unwrap_or_else(|| "QObject".to_string());
        generated.blocks.base_classes.push(base_class.clone());

        // Add the CxxQtType rust and rust_mut methods
        generated
            .blocks
            .append(&mut cxxqttype::generate(&qobject_idents)?);

        // Generate methods for the properties, invokables, signals
        generated.blocks.append(&mut generate_cpp_properties(
            &qobject.properties,
            &qobject_idents,
            cxx_mappings,
        )?);
        generated.blocks.append(&mut generate_cpp_methods(
            &qobject.methods,
            &qobject_idents,
            cxx_mappings,
        )?);
        generated.blocks.append(&mut generate_cpp_signals(
            &qobject.signals,
            &qobject_idents,
            cxx_mappings,
        )?);
        generated.blocks.append(&mut inherit::generate(
            &qobject.inherited_methods,
            &qobject.base_class,
            cxx_mappings,
        )?);
        generated
            .blocks
            .append(&mut qenum::generate(&qobject.qenums, cxx_mappings)?);

        let mut class_initializers = vec![];

        // If this type has threading enabled then add generation
        //
        // Note that threading also includes locking C++ generation
        if qobject.threading {
            // The parser phase should check that this is true
            debug_assert!(qobject.locking);

            let (initializer, mut blocks) = threading::generate(&qobject_idents)?;
            generated.blocks.append(&mut blocks);
            class_initializers.push(initializer);
        // If this type has locking enabled then add generation
        } else if qobject.locking {
            let (initializer, mut blocks) = locking::generate()?;
            generated.blocks.append(&mut blocks);
            class_initializers.push(initializer);
        }

        generated.blocks.append(&mut constructor::generate(
            &generated,
            &qobject.constructors,
            base_class,
            &class_initializers,
            cxx_mappings,
        )?);

        Ok(generated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::Parser;
    use syn::{parse_quote, ItemMod};

    #[test]
    fn test_generated_cpp_qobject_blocks() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &ParsedCxxMappings::default(),
        )
        .unwrap();
        assert_eq!(cpp.ident, "MyObject");
        assert_eq!(cpp.rust_ident, "MyObjectRust");
        assert_eq!(cpp.namespace_internals, "cxx_qt_my_object");

        assert_eq!(cpp.blocks.base_classes.len(), 3);
        assert_eq!(cpp.blocks.base_classes[0], "QObject");
        assert_eq!(
            cpp.blocks.base_classes[1],
            "::rust::cxxqtlib1::CxxQtType<MyObjectRust>"
        );
        assert_eq!(
            cpp.blocks.base_classes[2],
            "::rust::cxxqtlib1::CxxQtLocking"
        );
        assert_eq!(cpp.blocks.metaobjects.len(), 0);
    }

    #[test]
    fn test_generated_cpp_qobject_blocks_base_and_namespace() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    #[base = "QStringListModel"]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &ParsedCxxMappings::default(),
        )
        .unwrap();
        assert_eq!(cpp.namespace_internals, "cxx_qt::cxx_qt_my_object");
        assert_eq!(cpp.blocks.base_classes.len(), 3);
        assert_eq!(cpp.blocks.base_classes[0], "QStringListModel");
        assert_eq!(
            cpp.blocks.base_classes[1],
            "::rust::cxxqtlib1::CxxQtType<MyObjectRust>"
        );
        assert_eq!(
            cpp.blocks.base_classes[2],
            "::rust::cxxqtlib1::CxxQtLocking"
        );
        assert_eq!(cpp.blocks.metaobjects.len(), 0);
    }

    #[test]
    fn test_generated_cpp_qobject_named() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    #[qml_element = "MyQmlElement"]
                    type MyNamedObject = super::MyNamedObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &ParsedCxxMappings::default(),
        )
        .unwrap();
        assert_eq!(cpp.ident, "MyNamedObject");
        assert_eq!(cpp.blocks.metaobjects.len(), 1);
        assert_eq!(
            cpp.blocks.metaobjects[0],
            "Q_CLASSINFO(\"QML.Element\", \"MyQmlElement\")"
        );
    }

    #[test]
    fn test_generated_cpp_qobject_singleton() {
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

        let cpp = GeneratedCppQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &ParsedCxxMappings::default(),
        )
        .unwrap();
        assert_eq!(cpp.ident, "MyObject");
        assert_eq!(cpp.blocks.metaobjects.len(), 2);
        assert_eq!(
            cpp.blocks.metaobjects[0],
            "Q_CLASSINFO(\"QML.Element\", \"MyObject\")"
        );
        assert_eq!(cpp.blocks.metaobjects[1], "QML_SINGLETON");
    }

    #[test]
    fn test_generated_cpp_qobject_uncreatable() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    #[qml_element]
                    #[qml_uncreatable]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppQObject::from(
            parser.cxx_qt_data.qobjects.values().next().unwrap(),
            &ParsedCxxMappings::default(),
        )
        .unwrap();
        assert_eq!(cpp.ident, "MyObject");
        assert_eq!(cpp.blocks.metaobjects.len(), 2);
        assert_eq!(
            cpp.blocks.metaobjects[0],
            "Q_CLASSINFO(\"QML.Element\", \"MyObject\")"
        );
        assert_eq!(
            cpp.blocks.metaobjects[1],
            "Q_CLASSINFO(\"QML.Creatable\", \"false\")"
        );
    }
}
