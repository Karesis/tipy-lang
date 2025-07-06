# tipy

## 一种有品位的、更舒适的 C 语言

> Tipy is a statically-typed, compiled programming language, designed with a core philosophy: to be a **more tasteful, comfortable C**.

`tipy` 旨在融合 C 语言的底层控制能力和性能，与现代语言的人体工程学及安全性。我希望在保留手动内存管理等底层特性的同时，提供一种更富有表现力、更不容易出错的编程体验。

告别头文件（`#include <headache.h>`），告别 `NULL` 和悬垂指针，`tipy` 致力于提供一种独特的、简洁且一致的“品味 (taste)”。

---

## ✨ 项目状态 (Project Status)

**当前版本: v0.0.1-alpha**

本项目正处于早期的 Alpha 开发阶段。我刚刚完成了一个巨大的里程碑：

* ✅ **v0.0.1**: 成功实现 "Hello, World!" 的端到端编译！
    * 手写了词法分析器、语法分析器，构建了 AST。
    * 通过 `inkwell` (LLVM wrapper) 生成了功能正确的 LLVM IR。
    * 成功将 LLVM IR 编译、链接为原生可执行文件。

---

## 👋 Hello, Tipy World!

这是 `tipy` 语言 v0.0.1 的样子：

```tipy
// hello.tp
main() {
    print("Hello, Tipy World!")
}
```

编译器 (`func`) 会将上述代码编译成如下的 LLVM 中间表示 (IR)：

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

## 🗺️ 路线图 (Roadmap)

我正在向着 `v0.1.0` 的目标前进，计划实现以下核心功能：

* [x] **v0.0.1: Hello, World!** (已完成)
* [ ] **变量系统**: 变量声明、赋值、默认不可变性 (`~` 标记可变)。
* [ ] **原生类型**: 整数、浮点数、布尔、字符、字符串。
* [ ] **控制流**: `if-else` 表达式、`loop`、`while`。
* [ ] **函数**: 完整的函数定义、参数传递和返回值。
* [ ] **类与枚举**: 初步的 `class` 和 `enum` 定义。
* [ ] **内存管理**: 手动内存管理 (`new`, `free`) 及相关的安全保证。
* [ ] **指针**: 独特的 `^` 指针语法和类型系统。

---

## 🚀 如何构建与运行 (v0.0.1)

**先决条件:**
1.  **Rust 工具链**: 前往 [rust-lang.org](https://rust-lang.org) 安装。
2.  **LLVM 工具链**: 需要与 `inkwell` 依赖版本匹配的 LLVM。本项目当前使用 **LLVM 18**。
    ```bash
    # 以 Ubuntu 为例
    sudo apt install llvm-18 clang-18
    ```

**构建与运行步骤:**

1.  **运行你的编译器**:
    这一步会编译并运行 `tipyc` 本身，它会读取内置的 "Hello, World!" 源码，并生成 `output.ll` 文件。
    ```bash
    cargo run
    ```

2.  **将 LLVM IR 编译为目标文件**:
    使用 `llc` 将人类可读的 IR 转换为二进制目标文件。
    ```bash
    llc-18 -filetype=obj -relocation-model=pic -o output.o output.ll
    ```

3.  **链接为可执行文件**:
    使用 `clang` 将你的目标文件与系统C库（`puts` 函数在这里）链接起来。
    ```bash
    clang-18 output.o -o my_program
    ```

4.  **运行你的第一个 tipy 程序!**
    ```bash
    ./my_program
    # 输出: Hello, Tipy World!
    ```

---

## 🤝 如何贡献 (How to Contribute)

我非常欢迎任何形式的贡献！无论是提交 issue、修复 bug、实现新功能，还是改进文档，都对这个项目至关重要。

1.  Fork 本仓库。
2.  创建你的功能分支 (`git checkout -b feature/AmazingFeature`)。
3.  提交你的改动 (`git commit -m 'Add some AmazingFeature'`)。
4.  推送到分支 (`git push origin feature/AmazingFeature`)。
5.  提交一个 Pull Request。

---

## 📝 许可 (License)

本项目采用 [MIT 许可证](LICENSE.md)授权。