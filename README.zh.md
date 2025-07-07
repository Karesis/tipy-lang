# Tipy

## 一种有品位的、更舒适的 C 语言

> Tipy is a statically-typed, compiled programming language, designed with a core philosophy: to be a **more tasteful, comfortable C**.

`tipy` 旨在融合 C 语言的底层控制能力和性能，与现代语言的人体工程学及安全性。我希望在保留手动内存管理等底层特性的同时，提供一种更富有表现力、更不容易出错的编程体验。

告别头文件（`#include <headache.h>`），告别 `NULL` 和悬垂指针，`tipy` 致力于提供一种独特的、简洁且一致的“品味 (taste)”。

-----

## ✨ 项目状态 (Project Status)

**当前版本: v0.0.3-alpha**

本项目正处于早期的 Alpha 开发阶段。我们刚刚完成了一系列激动人心的里程碑，为 Tipy 的未来奠定了坚实的基础！

  * ✅ **v0.0.1: Hello, World\!**

      * 成功实现 "Hello, World\!" 的端到端编译，打通了从源代码到原生可执行文件的完整流程。

  * ✅ **v0.0.2: 做点数学 (Do Some Maths\!)**

      * 引入了变量声明 (`:`)、赋值 (`=`) 和可变性标记 (`~`)。
      * 支持了 `i32` 和 `f64` 原生类型及基本的四则运算。
      * 实现了 Pratt Parser 来正确处理运算符优先级。

  * ✅ **v0.0.3: 函数是一等公民 (Functions are First-Class)**

      * 实现了包含参数和返回值的完整函数定义语法 (`->`)。
      * 重构并实现了**两遍式语义分析 (Two-Pass Semantic Analysis)**，支持函数间的相互调用和前向引用。
      * 建立了基于栈的作用域和符号表，正确处理函数参数和局部变量。
      * 代码生成器 (`CodeGen`) 已完全重构，能够为函数、变量和表达式生成正确的 LLVM IR。

-----

## 👋 Hello, Tipy World\! (v0.0.3 Style)

随着函数系统的完善，`tipy` 现在能做更多事情了！

```tipy
// main.tp
add(a: i32, b: i32) -> i32 {
    ret a + b
}

main() -> i32 {
    a: i32 = 10;
    b: ~i32 = 20;
    b = add(a, b * 2); // 应该计算为 add(10, 40) = 50
    ret b;
}
```

我们的编译器 (`tipyc`) 会将上述代码编译成类似下面这样的 LLVM 中间表示 (IR)。可以看到，它已经包含了两个函数定义、栈分配 (`alloca`)、加载/存储 (`load`/`store`)、运算和函数调用 (`call`)：

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

## 🗺️ 路线图 (Roadmap)

我正在向着 `v0.1.0` 的目标前进，计划实现以下核心功能：

  * [x] **v0.0.1: Hello, World\!**
  * [x] **v0.0.2: 变量与运算 (Variables & Operations)**
      * [x] 变量声明与赋值
      * [x] 原生类型 (`i32`, `f64`) & 基本算术
  * [x] **v0.0.3: 函数系统 (Function System)**
      * [x] 完整函数定义、参数、返回值
      * [x] 作用域与符号表
      * [x] 端到端函数调用
  * [ ] **控制流**: `if-else` 表达式、`loop`、`while`。
  * [ ] **原生类型扩展**: 布尔、字符、字符串。
  * [ ] **类与枚举**: 初步的 `class` 和 `enum` 定义。
  * [ ] **内存管理**: 手动内存管理 (`new`, `free`) 及相关的安全保证。
  * [ ] **指针**: 独特的 `^` 指针语法和类型系统。

-----

## 🚀 如何构建与运行 (v0.0.3)

**先决条件:**

1.  **Rust 工具链**: 前往 [rust-lang.org](https://rust-lang.org) 安装。
2.  **LLVM 工具链**: 需要与 `inkwell` 依赖版本匹配的 LLVM。本项目当前使用 **LLVM 18**。
    ```bash
    # 以 Ubuntu 为例
    sudo apt install llvm-18 clang-18
    ```

**构建与运行步骤:**

1.  **运行你的编译器**:
    这一步会编译并运行 `tipyc` (即您的 `func` 二进制文件) 本身，它会读取内置的 Tipy 源码，并生成 `output.ll` 文件。

    ```bash
    cargo run
    ```

2.  **将 LLVM IR 编译为目标文件**:
    使用 `llc` 将人类可读的 IR 转换为二进制目标文件。

    ```bash
    llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll
    ```

3.  **链接为可执行文件**:
    使用 `clang` 将你的目标文件链接起来。

    ```bash
    clang-18 output.o -o my_program
    ```

4.  **运行你的第一个 Tipy 程序\!**

    ```bash
    ./my_program
    ```

5.  **检查程序返回值**:
    我们的 `main` 函数 `ret b;`，其最终值是 `50`。在 Linux 或 macOS 上，你可以用 `echo $?` 来检查上一个命令的退出码。

    ```bash
    echo $?
    # 输出: 50
    ```

-----

## 🤝 如何贡献 (How to Contribute)

我非常欢迎任何形式的贡献！无论是提交 issue、修复 bug、实现新功能，还是改进文档，都对这个项目至关重要。

1.  Fork 本仓库。
2.  创建你的功能分支 (`git checkout -b feature/AmazingFeature`)。
3.  提交你的改动 (`git commit -m 'Add some AmazingFeature'`)。
4.  推送到分支 (`git push origin feature/AmazingFeature`)。
5.  提交一个 Pull Request。

-----

## 📝 许可 (License)

本项目采用 [Apache-2.0 许可证](https://www.google.com/search?q=LICENSE)授权。