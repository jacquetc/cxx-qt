// SPDX-FileCopyrightText: 2023 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Leon Matthes <leon.matthes@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::qobject::GeneratedCppQObjectBlocks;
use crate::{
    generator::{cpp::GeneratedCppQObject, utils::cpp::syn_type_to_cpp_type},
    parser::{constructor::Constructor, mappings::ParsedCxxMappings},
    CppFragment,
};

use indoc::formatdoc;
use syn::{Result, Type};

fn default_constructor(
    qobject: &GeneratedCppQObject,
    base_class: String,
    initializers: String,
) -> GeneratedCppQObjectBlocks {
    GeneratedCppQObjectBlocks {
        methods: vec![CppFragment::Pair {
            header: format!(
                "explicit {class_name}(QObject* parent = nullptr);",
                class_name = qobject.ident
            ),
            source: formatdoc!(
                r#"
            {class_name}::{class_name}(QObject* parent)
              : {base_class}(parent)
              , ::rust::cxxqtlib1::CxxQtType<{rust_obj}>(::{namespace_internals}::createRs()){initializers}
            {{ }}
            "#,
                class_name = qobject.ident,
                namespace_internals = qobject.namespace_internals,
                rust_obj = qobject.rust_ident,
            ),
        }],
        ..Default::default()
    }
}

fn argument_names(arguments: &[Type]) -> Vec<String> {
    arguments
        .iter()
        .enumerate()
        .map(|(index, _)| format!("arg{index}"))
        .collect()
}

fn expand_arguments(arguments: &[Type], cxx_mappings: &ParsedCxxMappings) -> Result<String> {
    Ok(arguments
        .iter()
        .zip(argument_names(arguments).into_iter())
        .map(|(ty, name)| syn_type_to_cpp_type(ty, cxx_mappings).map(|ty| format!("{ty} {name}")))
        .collect::<Result<Vec<_>>>()?
        .join(", "))
}

pub fn generate(
    qobject: &GeneratedCppQObject,
    constructors: &[Constructor],
    base_class: String,
    class_initializers: &[String],
    cxx_mappings: &ParsedCxxMappings,
) -> Result<GeneratedCppQObjectBlocks> {
    let initializers = class_initializers
        .iter()
        .map(|initializer| format!("\n  , {initializer}"))
        .collect::<Vec<_>>()
        .join("");

    if constructors.is_empty() {
        return Ok(default_constructor(qobject, base_class, initializers));
    }

    let mut generated = GeneratedCppQObjectBlocks::default();

    let class_name = qobject.ident.as_str();
    let rust_obj = qobject.rust_ident.as_str();
    let namespace_internals = &qobject.namespace_internals;
    for (index, constructor) in constructors.iter().enumerate() {
        let argument_list = expand_arguments(&constructor.arguments, cxx_mappings)?;
        let constructor_argument_names = argument_names(&constructor.arguments);

        generated.methods.push(CppFragment::Pair {
            header: format!("explicit {class_name}({argument_list});"),
            source: formatdoc! {
                r#"
                {class_name}::{class_name}({argument_list})
                  : {class_name}(::{namespace_internals}::routeArguments{index}({move_arguments}))
                {{ }}
                "#,
                move_arguments = constructor_argument_names.iter().map(|arg| format!("::std::move({arg})")).collect::<Vec<_>>().join(", "),
            },
        });

        let base_args = if !constructor.base_arguments.is_empty() {
            argument_names(&constructor.base_arguments)
                .into_iter()
                .map(|arg| format!("::std::move(args.base.{arg})"))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            "".to_string()
        };
        // For each constructor defined in CXX-Qt we need a pair of one public and one private
        // constructor.
        // The reason for this is that CXX-Qt needs to be able to route the list of raw arguments
        // provided in C++ to a Plain-Old-Data type that contains the arguments already routed
        // through Rust.
        // This second constructor which takes the routed arguments is private, so that only CXX-Qt
        // can use it.
        generated.private_methods.push(CppFragment::Pair {
            header: format!(
                "explicit {class_name}(::{namespace_internals}::CxxQtConstructorArguments{index}&& args);"
            ),
            source: formatdoc! {
                r#"
                {class_name}::{class_name}(::{namespace_internals}::CxxQtConstructorArguments{index}&& args)
                  : {base_class}({base_args})
                  , ::rust::cxxqtlib1::CxxQtType<{rust_obj}>(::{namespace_internals}::newRs{index}(::std::move(args.new_))){initializers}
                {{
                  ::{namespace_internals}::initialize{index}(*this, ::std::move(args.initialize));
                }}
                "#,
            },
        })
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_eq;
    use syn::parse_quote;

    fn qobject_for_testing() -> GeneratedCppQObject {
        GeneratedCppQObject {
            ident: "MyObject".to_string(),
            rust_ident: "MyObjectRust".to_string(),
            namespace_internals: "rust".to_string(),
            blocks: GeneratedCppQObjectBlocks::default(),
        }
    }

    fn mock_constructor() -> Constructor {
        Constructor {
            arguments: vec![],
            base_arguments: vec![],
            new_arguments: vec![],
            initialize_arguments: vec![],
            lifetime: None,
            // dummy impl
            imp: parse_quote! { impl X {} },
        }
    }

    fn assert_empty_blocks(blocks: &GeneratedCppQObjectBlocks) {
        assert!(blocks.metaobjects.is_empty());
        assert!(blocks.forward_declares.is_empty());
    }

    #[test]
    fn default_constructor_with_initializers() {
        let blocks = generate(
            &qobject_for_testing(),
            &[],
            "BaseClass".to_owned(),
            &["member1(1)".to_string(), "member2{ 2 }".to_string()],
            &ParsedCxxMappings::default(),
        )
        .unwrap();

        assert_empty_blocks(&blocks);
        assert!(blocks.private_methods.is_empty());
        assert_eq!(
            blocks.methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(QObject* parent = nullptr);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(QObject* parent)
                      : BaseClass(parent)
                      , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::createRs())
                      , member1(1)
                      , member2{{ 2 }}
                    {{ }}
                    "
                ),
            }]
        );
    }
    #[test]
    fn default_constructor_without_initializers() {
        let blocks = generate(
            &qobject_for_testing(),
            &[],
            "BaseClass".to_owned(),
            &[],
            &ParsedCxxMappings::default(),
        )
        .unwrap();

        assert_empty_blocks(&blocks);
        assert!(blocks.private_methods.is_empty());
        assert_eq!(
            blocks.methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(QObject* parent = nullptr);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(QObject* parent)
                      : BaseClass(parent)
                      , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::createRs())
                    {{ }}
                    "
                ),
            }]
        );
    }

    #[test]
    fn constructor_without_base_arguments() {
        let blocks = generate(
            &qobject_for_testing(),
            &[Constructor {
                arguments: vec![parse_quote! { i32 }, parse_quote! { *mut QObject }],
                ..mock_constructor()
            }],
            "BaseClass".to_owned(),
            &[],
            &ParsedCxxMappings::default(),
        )
        .unwrap();

        assert_empty_blocks(&blocks);
        assert_eq!(
            blocks.private_methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(::rust::CxxQtConstructorArguments0&& args);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(::rust::CxxQtConstructorArguments0&& args)
                      : BaseClass()
                      , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::newRs0(::std::move(args.new_)))
                    {{
                      ::rust::initialize0(*this, ::std::move(args.initialize));
                    }}
                    "
                ),
            }]
        );
        assert_eq!(
            blocks.methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(::std::int32_t arg0, QObject* arg1);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(::std::int32_t arg0, QObject* arg1)
                      : MyObject(::rust::routeArguments0(::std::move(arg0), ::std::move(arg1)))
                    {{ }}
                    "
                ),
            }]
        );
    }

    #[test]
    fn constructor_with_all_arguments() {
        let blocks = generate(
            &qobject_for_testing(),
            &[Constructor {
                arguments: vec![parse_quote! { i8 }, parse_quote! { i16 }],
                new_arguments: vec![parse_quote! { i16}, parse_quote! { i32 }],
                initialize_arguments: vec![parse_quote! { i32 }, parse_quote! { i64 }],
                base_arguments: vec![parse_quote! { i64 }, parse_quote! { *mut QObject }],
                lifetime: Some(parse_quote! { 'a_lifetime }),
                ..mock_constructor()
            }],
            "BaseClass".to_owned(),
            &["initializer".to_string()],
            &ParsedCxxMappings::default(),
        )
        .unwrap();

        assert_empty_blocks(&blocks);
        assert_eq!(
            blocks.methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(::std::int8_t arg0, ::std::int16_t arg1);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(::std::int8_t arg0, ::std::int16_t arg1)
                      : MyObject(::rust::routeArguments0(::std::move(arg0), ::std::move(arg1)))
                    {{ }}
                    "
                )
            }]
        );
        assert_eq!(
            blocks.private_methods,
            vec![CppFragment::Pair {
                header: "explicit MyObject(::rust::CxxQtConstructorArguments0&& args);".to_string(),
                source: formatdoc!(
                    "
                    MyObject::MyObject(::rust::CxxQtConstructorArguments0&& args)
                      : BaseClass(::std::move(args.base.arg0), ::std::move(args.base.arg1))
                      , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::newRs0(::std::move(args.new_)))
                      , initializer
                    {{
                      ::rust::initialize0(*this, ::std::move(args.initialize));
                    }}
                    "
                )
            }]
        );
    }

    #[test]
    fn multiple_constructors() {
        let blocks = generate(
            &qobject_for_testing(),
            &[
                Constructor {
                    arguments: vec![],
                    ..mock_constructor()
                },
                Constructor {
                    arguments: vec![parse_quote! { *mut QObject }],
                    base_arguments: vec![parse_quote! { *mut QObject }],
                    ..mock_constructor()
                },
            ],
            "BaseClass".to_owned(),
            &["initializer".to_string()],
            &ParsedCxxMappings::default(),
        )
        .unwrap();

        assert_empty_blocks(&blocks);
        assert_eq!(blocks.methods.len(), 2);
        assert_eq!(
            blocks.methods,
            vec![
                CppFragment::Pair {
                    header: "explicit MyObject();".to_string(),
                    source: formatdoc!(
                        "
                        MyObject::MyObject()
                          : MyObject(::rust::routeArguments0())
                        {{ }}
                        "
                    ),
                },
                CppFragment::Pair {
                    header: "explicit MyObject(QObject* arg0);".to_string(),
                    source: formatdoc! {
                        "
                        MyObject::MyObject(QObject* arg0)
                          : MyObject(::rust::routeArguments1(::std::move(arg0)))
                        {{ }}
                        "
                    }
                }
            ]
        );
        assert_eq!(blocks.private_methods.len(), 2);
        assert_eq!(
            blocks.private_methods,
            vec![
                CppFragment::Pair {
                    header: "explicit MyObject(::rust::CxxQtConstructorArguments0&& args);"
                        .to_string(),
                    source: formatdoc!(
                        "
                        MyObject::MyObject(::rust::CxxQtConstructorArguments0&& args)
                          : BaseClass()
                          , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::newRs0(::std::move(args.new_)))
                          , initializer
                        {{
                          ::rust::initialize0(*this, ::std::move(args.initialize));
                        }}
                        "
                    )
                },
                CppFragment::Pair {
                    header: "explicit MyObject(::rust::CxxQtConstructorArguments1&& args);"
                        .to_string(),
                    source: formatdoc!(
                        "
                        MyObject::MyObject(::rust::CxxQtConstructorArguments1&& args)
                          : BaseClass(::std::move(args.base.arg0))
                          , ::rust::cxxqtlib1::CxxQtType<MyObjectRust>(::rust::newRs1(::std::move(args.new_)))
                          , initializer
                        {{
                          ::rust::initialize1(*this, ::std::move(args.initialize));
                        }}
                        "
                    )
                }
            ]
        );
    }
}
