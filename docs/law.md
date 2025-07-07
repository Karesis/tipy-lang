### Tipy 语言规范 (v0.0.0-alpha)

#### 1\. 引言与哲学

`tipy` 是一门静态类型的编译型编程语言，其核心哲学是成为一门\*\*“更舒适，更偷懒的C语言”\*\*。它旨在融合C语言的底层控制能力（如手动内存管理）与现代语言的安全性及人体工程学（如默认不可变性、无`null`保证、富有表现力的语法）。

`tipy` 不试图创造新的编程范式，而是为现有的、经过验证的范式提供一种独特的、简洁的、一致的“品味 (taste)”。

// 注：甚至，第一版的tipy会可能向着“对C动点手脚”的“纯C方言”的方向发展。

#### 2\. 词法结构

  * **2.1. 注释:**

      * 单行注释: `//`
      * 多行注释: `/* ... */` (暂定，MVP阶段可后置)

  * **2.2. 关键字 (Keywords):**
    `class`, `enum`, `match`, `if`, `else`, `loop`, `while`, `break`, `continue`, `ret`, `new`, `free`, `true`, `false`, `None`

  * **2.3. 标识符 (Identifiers):**
    以字母或下划线开头，后跟任意数量的字母、数字或下划线。例如 `my_var`, `Point`, `_internal`。

  * **2.4. 分隔符 (Delimiters):**

      * 花括号 `{}`: 定义代码块。
      * 圆括号 `()`: 函数调用、构造函数、表达式分组。
      * 分号 `;`: 可选。仅在一行内书写多条语句时需要。

#### 3\. 类型系统

  * **3.1. 原生类型 (Primitive Types):**

      * 有符号整数: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
      * 无符号整数: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
      * 浮点数: `f32`, `f64`
      * 布尔: `bool` (值为 `true` 或 `false`)
      * 字符: `char` (4字节Unicode标量值)
      * 字符串: `str` (内置的、自动管理的拥有型字符串)

  * **3.2. 指针类型 (Pointer Types):**
    使用 `^` 表示指针。详见第10节。

  * **3.3. 枚举类型 (Enum Types):**
    使用 `enum` 关键字定义代数数据类型。详见第9节。

#### 4\. 变量与可变性

  * **4.1. 默认不可变性:** `tipy` 中所有的变量绑定和 `class` 字段默认都是不可变的。

  * **4.2. 可变性标记 `~`:**
    使用 `~` 符号来显式声明一个绑定或字段是可变的。

      * **可变绑定:** `my_var: ~i32 = 10`，`my_var` 可以被重新赋值。
      * **可变字段:** `class Point(x: ~f64, y: ~f64)`，`Point` 实例的 `x` 和 `y` 字段可以被修改。

  * **4.3. 声明语法:**

      * `变量名: 类型`
      * `变量名: 类型 = 初始值`
      * `变量1, 变量2: 类型 = 初始值` (批量声明并初始化)

    <!-- end list -->

    ```tipy
    x: i32 = 10         // 不可变
    y: ~i32 = 20        // 可变
    name: str = "tipy"  // 不可变
    ```

#### 5\. 函数

  * **5.1. 定义:**

      * 函数定义的标识是 `->` 返回箭头。如果函数不返回值，则 `->` 和返回类型都可以省略。
      * `函数名(参数: 类型, ...) -> 返回类型 { ... }`

  * **5.2. 返回值:**

      * 函数的最后一个表达式是其隐式返回值。
      * `ret` 关键字用于从函数中提前返回。

    <!-- end list -->

    ```tipy
    // 有返回值
    add(a: i32, b: i32) -> i32 {
        a + b
    }

    // 无返回值 (隐式返回 void)
    print_sum(a: i32, b: i32) {
        print(a + b)
    }

    // 使用 ret 提前返回
    find(haystack: str, needle: char) -> i32 {
        // ... some logic ...
        if found {
            ret index
        }
        -1 // 隐式返回
    }
    ```

#### 6\. 控制流 (均为表达式)

  * **6.1. `if-else`:** 花括号强制。作为表达式使用时，`else` 分支强制。
    ```tipy
    value: i32 = if condition { 1 } else { 0 }
    ```
    链式调用则为 `if-elif-else`
  * **6.2. `loop`:** 无限循环，必须由 `break` 退出。`break` 可以带一个值作为 `loop` 表达式的返回值。
    ```tipy
    counter: ~i32 = 0
    result: i32 = loop {
        counter = counter + 1
        if counter == 10 {
            break counter * 2 // result 将被赋值为 20
        }
    }
    ```
  * **6.3. `while`:** `while` 是语句，不是表达式，本身不返回值。
  * **6.4. `match`:** 详见第9节。

#### 7\. 类 (Class)

  * **7.1. 定义:**
    采用“主构造函数”一体化语法。`class` 字段的可变性由 `~` 标记。

    ```tipy
    // x, y 字段不可变
    class Point(x: f64, y: f64)

    // name 可变, age 不可变
    class User(name: ~str, age: const i32)
    ```

  * **7.2. 继承:**
    使用冒号 `:` 表示继承。

    ```tipy
    class Animal(name: str)
    class Cat(name: str, lives: i32) : Animal { ... }
    ```

  * **7.3. 方法与访问:**

      * 方法在 `class` 的 `{}` 代码块中定义。
      * 在方法内部，可以使用前缀 `.` 作为 `self.` 的语法糖来访问实例成员。

    <!-- end list -->

    ```tipy
    class Vector(x: ~f64, y: ~f64) {
        scale(factor: f64) {
            .x = .x * factor
            .y = .y * factor
        }
    }
    ```

#### 8\. 枚举与模式匹配

  * **8.1. `enum` 定义:**
    使用 `|` 分隔不同的变体。支持泛型。
    ```tipy
    enum Option<T> { Some(T) | None }
    enum Color { Red | Green | Blue }
    ```
  * **8.2. `match` 表达式:**
    用于解构 `enum` 并执行相应代码。
    ```tipy
    value: Option<i32> = Some(10)
    unwrapped: i32 = match value {
        Some(v) => v
        None => -1
    }
    ```

#### 9\. 内存管理与指针

  * **9.1. 无 `null`:** `tipy` 语言没有 `null` 关键字。所有可能为空的指针必须用 `Option<^T>` 显式包裹。

  * **9.2. 指针语法 `^`:**

      * `^T`: 不可变指针，指向不可变数据。
      * `~^T`: **可变指针**，指向不可变数据。(指针自身可被重指向)
      * `^~T`: 不可变指针，指向**可变数据**。(可用于修改所指数据)
      * `~^~T`: 可变指针，指向可变数据。

  * **9.3. 内存分配 `new`:**
    `new` 关键字在堆上分配内存并返回一个指针。

    ```tipy
    p: ^Point = new Point(1.0, 2.0)
    ```

  * **9.4. 安全释放 `free`:**
    `free` 是一个语言构造。它只能作用于一个可变的 `Option` 指针变量上 (`~Option<^T>`)。它会安全地释放内存，并将该指针变量的值自动置为 `None`，从根本上防止悬垂指针和二次释放。

    ```tipy
    p_opt: ~Option<^Point> = Some(new Point(1.0, 1.0))
    free(p_opt) // 执行后, p_opt 的值变为 None
    ```

#### 10\. “Hello, World\!” 示例

```tipy
// hello.tp

// main 函数是程序的入口，不返回值
main() {
    // print 是一个内置或标准库函数
    print("Hello, Tipy World!")
}
```

