# Phase 4 – Ecosystem Crate: `blaze‑jni`

> **Goal:** Provide a safe, data‑oriented interface for calling Java code from Blaze and for writing Java native methods in Blaze.  It wraps the Java Native Interface (JNI) using Blaze’s FFI and linear type system, automatically handling class loading, method resolution, and type conversion.  The crate is designed for building Android applications, Java plugins, and inter‑language tooling.  All Java operations carry the `io` effect (since they involve a JVM).

---

## 1. Core Concepts

- **`Jvm`** – a handle to a running Java Virtual Machine.
- **`JClass`** – a reference to a Java class.
- **`JObject`** – a reference to a Java object.
- **`JMethodId`**, **`JFieldId`** – typed identifiers for methods and fields.
- **`JValue`** – an enum representing any Java value (primitive, object, or null).

All types are linear where they own a JNI global reference; `JObject` and `JClass` implement `Dispose` to release the reference.  The JNI environment pointer is hidden behind a safe wrapper.

---

## 2. `Jvm`

### 2.1 Creating / Obtaining a JVM

```
pub struct Jvm { … }
impl Jvm {
    pub fn new() -> Result<Jvm, JniError>;      // creates a new JVM (if not already attached)
    pub fn attach_current_thread() -> Result<Jvm, JniError>;  // attaches the current thread to an existing JVM
}
```

- `new` sets up a JVM with default options; `attach_current_thread` is used when Blaze is called from Java.

### 2.2 Class and Object Operations

```
impl Jvm {
    pub fn find_class(&self, name: &str) -> Result<JClass, JniError>;
    pub fn new_object(&self, class: &JClass, constructor_sig: &str, args: &[JValue]) -> Result<JObject, JniError>;
    pub fn call_method(&self, obj: &JObject, name: &str, sig: &str, args: &[JValue]) -> Result<JValue, JniError>;
    pub fn call_static_method(&self, class: &JClass, name: &str, sig: &str, args: &[JValue]) -> Result<JValue, JniError>;
    pub fn get_field(&self, obj: &JObject, name: &str, sig: &str) -> Result<JValue, JniError>;
    pub fn set_field(&self, obj: &JObject, name: &str, sig: &str, value: &JValue) -> Result<(), JniError>;
}
```

- Method and field signatures follow the JNI type descriptor format (e.g., `"(I)Ljava/lang/String;"`).

---

## 3. `JValue`

```
pub enum JValue {
    Bool(bool),
    Byte(i8),
    Char(u16),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Object(JObject),
    Null,
}
```

- Implements `From` for all primitive types and `From<JObject>`.
- `JObject` can be downcast to a concrete type via `find_class` + `is_assignable_from`.

---

## 4. Writing Native Methods (Java → Blaze)

The crate provides a `#[jni_export]` macro that turns a Blaze function into a JNI‑compatible native method.

```
#[jni_export]
fn Java_com_example_MyClass_add(env: &JniEnv, _class: &JClass, a: i32, b: i32) -> i32 {
    a + b
}
```

The macro generates the necessary `extern "C"` wrapper and JNI name mangling.  The `JniEnv` parameter is automatically provided.

---

## 5. Error Handling

```
pub enum JniError {
    JvmInitFailed,
    ClassNotFound,
    MethodNotFound,
    FieldNotFound,
    ExceptionThrown(Text),   // contains pending Java exception message
    InvalidArgument,
    Io(std::io::Error),
}
```

- If a Java exception is pending after a call, the crate clears it and returns `JniError::ExceptionThrown`.

---

## 6. Testing

- **Create a JVM:** Instantiate a JVM, call `System.getProperty("java.version")`, verify it returns a non‑null string.
- **Invoke a method:** Call `java.lang.Math.abs(-1)` and verify the result is `1`.
- **Native method:** Export a Blaze function, load it from a small Java test harness, and verify it returns the correct value.

Tests require a Java runtime installed on the machine.
