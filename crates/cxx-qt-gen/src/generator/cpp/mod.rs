// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod constructor;
pub mod cxxqttype;
pub mod externcxxqt;
pub mod fragment;
pub mod inherit;
pub mod locking;
pub mod method;
pub mod property;
pub mod qenum;
pub mod qobject;
pub mod signal;
pub mod threading;

use crate::parser::Parser;
use externcxxqt::GeneratedCppExternCxxQtBlocks;
use qobject::GeneratedCppQObject;
use syn::Result;

/// Representation of the generated C++ code for a group of QObjects
pub struct GeneratedCppBlocks {
    /// Stem of the CXX header to include
    pub cxx_file_stem: String,
    /// Ident of the common namespace of the QObjects
    pub namespace: String,
    /// Generated QObjects
    pub qobjects: Vec<GeneratedCppQObject>,
    /// Generated extern C++Qt blocks
    pub extern_cxx_qt: Vec<GeneratedCppExternCxxQtBlocks>,
}

impl GeneratedCppBlocks {
    pub fn from(parser: &Parser) -> Result<GeneratedCppBlocks> {
        Ok(GeneratedCppBlocks {
            cxx_file_stem: parser.cxx_file_stem.clone(),
            namespace: parser.cxx_qt_data.namespace.clone(),
            qobjects: parser
                .cxx_qt_data
                .qobjects
                .values()
                .map(|qobject| GeneratedCppQObject::from(qobject, &parser.cxx_qt_data.cxx_mappings))
                .collect::<Result<Vec<GeneratedCppQObject>>>()?,
            extern_cxx_qt: externcxxqt::generate(
                &parser.cxx_qt_data.extern_cxxqt_blocks,
                &parser.cxx_qt_data.cxx_mappings,
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::Parser;
    use syn::{parse_quote, ItemMod};

    #[test]
    fn test_generated_cpp_blocks() {
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

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.cxx_file_stem, "ffi");
        assert_eq!(cpp.namespace, "");
        assert_eq!(cpp.qobjects.len(), 1);
    }

    #[test]
    fn test_generated_cpp_blocks_cxx_file_stem() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(cxx_file_stem = "my_object")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.cxx_file_stem, "my_object");
        assert_eq!(cpp.namespace, "");
        assert_eq!(cpp.qobjects.len(), 1);
    }

    #[test]
    fn test_generated_cpp_blocks_namespace() {
        let module: ItemMod = parse_quote! {
            #[cxx_qt::bridge(namespace = "cxx_qt")]
            mod ffi {
                extern "RustQt" {
                    #[qobject]
                    type MyObject = super::MyObjectRust;
                }
            }
        };
        let parser = Parser::from(module).unwrap();

        let cpp = GeneratedCppBlocks::from(&parser).unwrap();
        assert_eq!(cpp.namespace, "cxx_qt");
    }
}
