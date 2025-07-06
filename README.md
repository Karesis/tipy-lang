# tipy

## A more tasteful, comfortable C.

> Tipy is a statically-typed, compiled programming language, designed with a core philosophy: to be a **more tasteful, comfortable C**.

`tipy` aims to fuse the low-level control and performance of C with the ergonomics and safety of modern languages. I want to provide a more expressive and less error-prone programming experience while retaining low-level features like manual memory management.

Say goodbye to header files (`#include <headache.h>`), say goodbye to `NULL` and dangling pointers. `tipy` is dedicated to providing a unique, concise, and consistent "taste".

---

## ✨ Project Status

**Current Version: v0.0.1-alpha**

This project is in its early alpha stage of development. I have just reached a major milestone:

* ✅ **v0.0.1**: Successfully achieved end-to-end compilation for a "Hello, World!" program!
    * Hand-written a lexer and parser to build an Abstract Syntax Tree (AST).
    * Generated functional LLVM IR via `inkwell` (a Rust wrapper for LLVM).
    * Successfully compiled and linked the LLVM IR into a native executable.

---

## 👋 Hello, Tipy World!

This is what `tipy` looks like in v0.0.1:

```tipy
// hello.tp
main() {
    print("Hello, Tipy World!")
}
```

My compiler (`tipyc`) will compile the code above into the following LLVM Intermediate Representation (IR):

```llvm
; ModuleID = 'tipy_module'
source_filename = "tipy_module"

@.str = private unnamed_addr constant [19 x i8] c"Hello,Tipy World!\00", align 1

declare i32 @puts(ptr)

define i32 @main() {
entry:
  %tmp_call = call i32 @puts(ptr @.str)
  ret i32 0
}
```

---

## 🗺️ Roadmap

I am moving towards the `v0.1.0` target, with plans to implement the following core features:

* [x] **v0.0.1: Hello, World!** (Completed)
* [ ] **Variable System**: Variable declarations, assignments, and default immutability (`~` for mutable).
* [ ] **Primitive Types**: Integers, floats, booleans, chars, strings.
* [ ] **Control Flow**: `if-else` expressions, `loop`, `while`.
* [ ] **Functions**: Full function definitions, parameters, and return values.
* [ ] **Classes & Enums**: Initial definitions for `class` and `enum`.
* [ ] **Memory Management**: Manual memory management (`new`, `free`) with related safety guarantees.
* [ ] **Pointers**: The unique `^` pointer syntax and type system.

---

## 🚀 Building and Running (v0.0.1)

**Prerequisites:**
1.  **Rust Toolchain**: Install it from [rust-lang.org](https://rust-lang.org).
2.  **LLVM Toolchain**: A version matching the `inkwell` dependency is required. This project currently uses **LLVM 18**.
    ```bash
    # Example for Ubuntu
    sudo apt install llvm-18 clang-18
    ```

**Build and Run Steps:**

1.  **Run Your Compiler**:
    This step compiles and runs `tipyc` itself. It will read the built-in "Hello, World!" source code and generate an `output.ll` file.
    ```bash
    cargo run
    ```

2.  **Compile LLVM IR to an Object File**:
    Use `llc` to convert the human-readable IR into a binary object file.
    ```bash
    llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll
    ```

3.  **Link into an Executable**:
    Use `clang` to link your object file with the system's C standard library (where the `puts` function lives).
    ```bash
    clang-18 output.o -o my_program
    ```

4.  **Run your first tipy program!**
    ```bash
    ./my_program
    # Output: Hello, Tipy World!
    ```

---

## 🤝 How to Contribute

Contributions of all kinds are welcome! Whether it's submitting issues, fixing bugs, implementing new features, or improving documentation, every bit of help is vital to this project.

1.  Fork the repository.
2.  Create your feature branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the branch (`git push origin feature/AmazingFeature`).
5.  Open a Pull Request.

---

## 📝 License

This project is licensed under the [Apache-2.0 License](LICENSE).