# Tipy

## A more tasteful, comfortable C.

> Tipy is a statically-typed, compiled programming language, designed with a core philosophy: to be a **more tasteful, comfortable C**.

`tipy` aims to fuse the low-level control and performance of C with the ergonomics and safety of modern languages. I want to provide a more expressive and less error-prone programming experience while retaining low-level features like manual memory management.

Say goodbye to header files (`#include <headache.h>`), say goodbye to `NULL` and dangling pointers. `tipy` is dedicated to providing a unique, concise, and consistent "taste".

-----

## ✨ Project Status

**Current Version: v0.0.3-alpha**

This project is in its early alpha stage of development. We've just completed a series of major milestones, laying a solid foundation for the future of Tipy\!

  * ✅ **v0.0.1: Hello, World\!**

      * Achieved end-to-end compilation for a "Hello, World\!" program, establishing the full pipeline from source code to a native executable.

  * ✅ **v0.0.2: Do Some Maths\!**

      * Introduced the variable system, including declarations (`:`), assignments (`=`), and the mutability marker (`~`).
      * Supported `i32` and `f64` primitive types with basic arithmetic operations.
      * Implemented a Pratt Parser to correctly handle operator precedence.

  * ✅ **v0.0.3: Functions are First-Class\!**

      * Implemented full function definition syntax with parameters and return types (`->`).
      * Refactored and implemented a **Two-Pass Semantic Analyzer** to support mutual recursion and forward references for functions.
      * Established a stack-based scope and symbol table system to correctly handle function arguments and local variables.
      * The `CodeGen` has been completely overhauled to generate correct LLVM IR for functions, variables, and expressions.

-----

## 👋 Hello, Tipy World\! (v0.0.3 Style)

With a proper function system, `tipy` can now do much more\!

```tipy
// main.tp
add(a: i32, b: i32) -> i32 {
    ret a + b
}

main() -> i32 {
    a: i32 = 10;
    b: ~i32 = 20;
    b = add(a, b * 2); // Should evaluate to add(10, 40) = 50
    ret b;
}
```

My compiler (`tipyc`) will compile the code above into the following LLVM Intermediate Representation (IR). As you can see, it now includes two function definitions, stack allocations (`alloca`), loads/stores, arithmetic, and function calls (`call`):

```llvm
; ModuleID = 'tipy_module'
source_filename = "tipy_module"

define i32 @add(i32 %a, i32 %b) {
entry:
  %a1 = alloca i32, align 4
  store i32 %a, ptr %a1, align 4
  %b2 = alloca i32, align 4
  store i32 %b, ptr %b2, align 4
  %tmp_load = load i32, ptr %a1, align 4
  %tmp_load3 = load i32, ptr %b2, align 4
  %tmpadd = add i32 %tmp_load, %tmp_load3
  ret i32 %tmpadd
}

define i32 @main() {
entry:
  %b = alloca i32, align 4
  %a = alloca i32, align 4
  store i32 10, ptr %a, align 4
  store i32 20, ptr %b, align 4
  %tmp_load = load i32, ptr %a, align 4
  %tmp_load1 = load i32, ptr %b, align 4
  %tmpmul = mul i32 %tmp_load1, 2
  %tmpcall = call i32 @add(i32 %tmp_load, i32 %tmpmul)
  store i32 %tmpcall, ptr %b, align 4
  %tmp_load2 = load i32, ptr %b, align 4
  ret i32 %tmp_load2
}
```

-----

## 🗺️ Roadmap

I am moving towards the `v0.1.0` target, with plans to implement the following core features:

  * [x] **v0.0.1: Hello, World\!**
  * [x] **v0.0.2: Variables & Operations**
      * [x] Variable Declaration & Assignment
      * [x] Primitive Types (`i32`, `f64`) & Basic Arithmetic
  * [x] **v0.0.3: Function System**
      * [x] Full Function Definitions, Parameters, & Return Values
      * [x] Scoping & Symbol Tables
      * [x] End-to-end Function Calls
  * [ ] **Control Flow**: `if-else` expressions, `loop`, `while`.
  * [ ] **Expanded Primitives**: Booleans, characters, strings.
  * [ ] **Classes & Enums**: Initial definitions for `class` and `enum`.
  * [ ] **Memory Management**: Manual memory management (`new`, `free`) with related safety guarantees.
  * [ ] **Pointers**: The unique `^` pointer syntax and type system.

-----

## 🚀 Building and Running (v0.0.3)

**Prerequisites:**

1.  **Rust Toolchain**: Install it from [rust-lang.org](https://rust-lang.org).
2.  **LLVM Toolchain**: A version matching the `inkwell` dependency is required. This project currently uses **LLVM 18**.
    ```bash
    # Example for Ubuntu
    sudo apt install llvm-18 clang-18
    ```

**Build and Run Steps:**

1.  **Run Your Compiler**:
    This step compiles and runs `tipyc` (your `func` binary) itself. It will read the built-in Tipy source code and generate an `output.ll` file.

    ```bash
    cargo run
    ```

2.  **Compile LLVM IR to an Object File**:
    Use `llc` to convert the human-readable IR into a binary object file.

    ```bash
    llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll
    ```

3.  **Link into an Executable**:
    Use `clang` to link your object file.

    ```bash
    clang-18 output.o -o my_program
    ```

4.  **Run your Tipy program\!**

    ```bash
    ./my_program
    ```

5.  **Check the Return Code**:
    Our `main` function now returns the final value of `b`, which is `50`. On Linux or macOS, you can check the exit code of the last command with `echo $?`.

    ```bash
    echo $?
    # Expected output: 50
    ```

-----

## 🤝 How to Contribute

Contributions of all kinds are welcome\! Whether it's submitting issues, fixing bugs, implementing new features, or improving documentation, every bit of help is vital to this project.

1.  Fork the repository.
2.  Create your feature branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the branch (`git push origin feature/AmazingFeature`).
5.  Open a Pull Request.

-----

## 📝 License

This project is licensed under the [Apache-2.0 License](https://www.google.com/search?q=LICENSE).