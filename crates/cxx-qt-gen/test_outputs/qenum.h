#pragma once

#include <cstdint>
#include <cxx-qt-common/cxxqt_locking.h>
#include <cxx-qt-common/cxxqt_maybelockguard.h>
#include <cxx-qt-common/cxxqt_type.h>

namespace cxx_qt::my_object {
class MyObject;
enum class MyEnum : ::std::int32_t
{
  A
};

enum class MyOtherEnum : ::std::int32_t
{
  X,
  Y,
  Z
};

} // namespace cxx_qt::my_object

#include "cxx-qt-gen/ffi.cxx.h"

namespace cxx_qt::my_object {
class MyObject
  : public QObject
  , public ::rust::cxxqtlib1::CxxQtType<MyObjectRust>
  , public ::rust::cxxqtlib1::CxxQtLocking
{
  Q_OBJECT
public:
#ifdef Q_MOC_RUN
  enum class MyEnum : ::std::int32_t{ A };
  Q_ENUM(MyEnum)
#else
  using MyEnum = ::cxx_qt::my_object::MyEnum;
  Q_ENUM(MyEnum)
#endif

#ifdef Q_MOC_RUN
  enum class MyOtherEnum : ::std::int32_t{ X, Y, Z };
  Q_ENUM(MyOtherEnum)
#else
  using MyOtherEnum = ::cxx_qt::my_object::MyOtherEnum;
  Q_ENUM(MyOtherEnum)
#endif

  virtual ~MyObject() = default;

public:
  Q_INVOKABLE void myInvokable(
    ::cxx_qt::my_object::MyEnum qenum,
    ::cxx_qt::my_object::MyOtherEnum other_qenum) const;
  explicit MyObject(QObject* parent = nullptr);

private:
  void myInvokableWrapper(
    ::cxx_qt::my_object::MyEnum qenum,
    ::cxx_qt::my_object::MyOtherEnum other_qenum) const noexcept;
};

static_assert(::std::is_base_of<QObject, MyObject>::value,
              "MyObject must inherit from QObject");
} // namespace cxx_qt::my_object

Q_DECLARE_METATYPE(cxx_qt::my_object::MyObject*)
