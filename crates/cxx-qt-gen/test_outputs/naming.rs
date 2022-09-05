#[cxx::bridge(namespace = "")]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-gen/include/my_object.cxxqt.h");

        #[cxx_name = "MyObject"]
        type MyObjectQt;

        #[rust_name = "emit_property_name_changed"]
        fn emitPropertyNameChanged(self: Pin<&mut MyObjectQt>);
    }

    extern "Rust" {
        #[cxx_name = "MyObjectRust"]
        type MyObject;

        #[cxx_name = "invokableNameWrapper"]
        fn invokable_name_wrapper(self: &mut MyObject, cpp: Pin<&mut MyObjectQt>);

        #[cxx_name = "getPropertyName"]
        unsafe fn get_property_name<'a>(self: &'a MyObject, cpp: &'a MyObjectQt) -> &'a i32;
        #[cxx_name = "setPropertyName"]
        fn set_property_name(self: &mut MyObject, cpp: Pin<&mut MyObjectQt>, value: i32);
    }

    unsafe extern "C++" {
        include ! (< QtCore / QStringListModel >);
    }

    unsafe extern "C++" {
        include ! (< QtCore / QObject >);
        include!("cxx-qt-lib/include/convert.h");
        include!("cxx-qt-lib/include/cxxqt_thread.h");

        type MyObjectCxxQtThread;

        #[cxx_name = "unsafeRust"]
        fn rust(self: &MyObjectQt) -> &MyObject;

        #[cxx_name = "qtThread"]
        fn qt_thread(self: &MyObjectQt) -> UniquePtr<MyObjectCxxQtThread>;
        fn queue(self: &MyObjectCxxQtThread, func: fn(ctx: Pin<&mut MyObjectQt>)) -> Result<()>;

        #[rust_name = "new_cpp_object"]
        #[namespace = "cxx_qt_my_object"]
        fn newCppObject() -> UniquePtr<MyObjectQt>;
    }

    extern "C++" {
        #[cxx_name = "unsafeRustMut"]
        unsafe fn rust_mut(self: Pin<&mut MyObjectQt>) -> Pin<&mut MyObject>;
    }

    extern "Rust" {
        #[cxx_name = "createRs"]
        #[namespace = "cxx_qt_my_object"]
        fn create_rs() -> Box<MyObject>;
    }
}

pub use self::cxx_qt_ffi::*;
mod cxx_qt_ffi {
    use super::ffi::*;

    pub type FFICppObj = super::ffi::MyObjectQt;
    type UniquePtr<T> = cxx::UniquePtr<T>;

    unsafe impl Send for MyObjectCxxQtThread {}

    use std::pin::Pin;

    #[derive(Default)]
    pub struct MyObject {
        property_name: i32,
    }

    impl MyObject {
        pub fn get_property_name<'a>(&'a self, cpp: &'a MyObjectQt) -> &'a i32 {
            cpp.get_property_name()
        }

        pub fn set_property_name(&mut self, cpp: Pin<&mut MyObjectQt>, value: i32) {
            cpp.set_property_name(value);
        }

        pub fn invokable_name_wrapper(&mut self, cpp: std::pin::Pin<&mut FFICppObj>) {
            cpp.invokable_name();
        }
    }

    impl MyObjectQt {
        pub fn get_property_name(&self) -> &i32 {
            &self.rust().property_name
        }

        pub fn set_property_name(mut self: Pin<&mut Self>, value: i32) {
            unsafe {
                self.as_mut().rust_mut().property_name = value;
            }
            self.as_mut().emit_property_name_changed();
        }

        pub fn invokable_name(self: Pin<&mut Self>) {
            println!("Bye from Rust!");
            self.as_mut().set_property_name(5);
        }
    }

    pub fn create_rs() -> std::boxed::Box<MyObject> {
        std::default::Default::default()
    }
}
