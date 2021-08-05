mod my_object {
    use super::MyTrait;

    #[cxx::bridge]
    mod ffi {
        unsafe extern "C++" {
            include!("cxx-qt-gen/include/my_object.h");

            type MyObject;
            type QString = cxx_qt_lib::QString;

            #[rust_name = "number"]
            fn getNumber(self: &MyObject) -> i32;
            #[rust_name = "set_number"]
            fn setNumber(self: Pin<&mut MyObject>, value: i32);

            #[rust_name = "new_MyObject"]
            fn newMyObject() -> UniquePtr<MyObject>;
        }

        extern "Rust" {
            type MyObjectRs;

            #[cxx_name = "invokable"]
            fn invokable(self: &MyObjectRs);

            #[cxx_name = "createMyObjectRs"]
            fn create_my_object_rs() -> Box<MyObjectRs>;
        }
    }

    pub type CppObj = ffi::MyObject;

    struct MyObjectRs {
        number: i32,
    }

    impl MyObjectRs {
        fn invokable(&self) {}
    }

    struct MyObjectWrapper<'a> {
        cpp: std::pin::Pin<&'a mut CppObj>,
    }

    impl<'a> MyObjectWrapper<'a> {
        fn new(cpp: std::pin::Pin<&'a mut CppObj>) -> Self {
            Self { cpp }
        }

        fn number(&self) -> i32 {
            self.cpp.number()
        }

        fn set_number(&mut self, value: i32) {
            self.cpp.as_mut().set_number(value);
        }
    }

    struct MyObjectData {
        number: i32,
    }

    impl From<MyObjectData> for MyObjectRs {
        fn from(value: MyObjectData) -> Self {
            Self {
                number: value.number,
            }
        }
    }

    impl From<&MyObjectRs> for MyObjectData {
        fn from(value: &MyObjectRs) -> Self {
            Self {
                number: value.number.clone(),
            }
        }
    }

    impl Default for MyObjectRs {
        fn default() -> Self {
            Self { number: 32 }
        }
    }

    impl MyTrait for MyObjectRs {
        fn my_func() -> String {
            "Hello".to_owned()
        }
    }

    fn create_my_object_rs() -> Box<MyObjectRs> {
        Box::new(MyObjectRs::default())
    }
}
