// file: src/analyzer.rs

// --- 模块引入 ---

// 引入字面量用于分析
use crate::token::Literal;

// 引入诊断模块，用于将分析阶段发现的语义错误添加到错误收集中。
use crate::diagnostics::{CompilerError, SemanticError, Span};

// 引入抽象语法树 (AST) 模块。
// 语义分析器的主要工作就是遍历这些 AST 节点。
use crate::ast::{
    // --- 顶层结构 ---
    Program,
    TopLevelStatement,
    FunctionDeclaration,

    // --- 语句 (Statements) ---
    Statement,
    BlockStatement,
    VarDeclaration,
    ReturnStatement,
    WhileStatement,
    BreakStatement,
    ContinueStatement,
    
    // --- 表达式 (Expressions) ---
    Expression,
    IfExpression,
    LoopExpression,
    CallExpression,
    AssignmentExpression,
    PrefixExpression,
    InfixExpression,

    // --- 运算符 ---
    Operator,
    PrefixOperator,
};

// 引入作用域和符号管理模块。
// `SymbolTable` 是语义分析器用来跟踪变量和函数定义的核心数据结构。
use crate::scope::{Symbol, SymbolTable};

// 引入内部类型系统。
// `Type` 枚举用于表示变量、表达式和函数返回值的类型。
use crate::types::Type;


/// 语义分析器结构体。
///
/// 这是编译器的“大脑”，负责执行类型检查、作用域分析以及其他所有
/// 超越了纯粹语法的检查。它会遍历由 `Parser` 生成的 AST，
/// 并利用 `SymbolTable` 来验证代码的语义是否正确。
///
/// 例如，它会检查：
/// - 变量是否在使用前已被声明。
/// - 函数调用的参数数量和类型是否正确。
/// - 运算符两边的类型是否兼容（例如，不能将字符串和整数相加）。
/// - `break` 或 `continue` 是否只在循环内部使用。
pub struct SemanticAnalyzer {
    /// 符号表，用于管理所有作用域和在其中定义的符号（变量、函数等）。
    pub symbol_table: SymbolTable,
    
    /// 错误收集器。
    ///
    /// CHANGED: 类型从 `Vec<String>` 更新为 `Vec<CompilerError>`。
    /// 这使得语义分析器可以和词法、语法分析器一样，报告结构化的、
    /// 可携带位置信息的错误，完全融入了我们统一的诊断系统。
    pub errors: Vec<CompilerError>,
    
    /// 当前正在分析的函数的返回类型。
    ///
    /// 当分析器进入一个函数体时，这里会存下该函数的期望返回类型。
    /// 当遇到 `ret` 语句时，就可以用它来检查返回值的类型是否匹配。
    /// 当不在任何函数内部时，它的值是 `None`。
    current_return_type: Option<Type>,
    
    /// 当前所处的循环深度。
    ///
    /// - `0` 表示当前不在任何循环内部。
    /// - `1` 表示在最外层循环内。
    /// - `> 1` 表示在嵌套循环内。
    ///
    /// 这个计数器使得我们可以轻松地验证 `break` 和 `continue` 语句
    /// 是否被合法地使用在循环体中。
    loop_depth: u32,
}

impl SemanticAnalyzer {
    /// 创建一个新的、处于初始状态的 `SemanticAnalyzer` 实例。
    ///
    /// # Returns
    ///
    /// 一个全新的 `SemanticAnalyzer`，其内部包含一个已经初始化好的、
    /// 带有全局作用域的 `SymbolTable`，一个空的错误收集器，
    /// 并且没有预设的当前函数返回类型或循环深度。
    ///
    /// # Examples
    ///
    /// ```
    /// let mut analyzer = SemanticAnalyzer::new();
    /// ```
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            current_return_type: None,
            loop_depth: 0,
        }
    }

    /// 对给定的程序 AST (`Program`) 进行完整的语义分析。
    ///
    /// 这是语义分析阶段的唯一入口点。它采用“两遍式分析”策略，以正确处理
    /// 前向引用（例如，一个函数调用在它的定义之前出现）。
    ///
    /// **第一遍 (Pass 1): 符号注册**
    /// 遍历所有顶层声明，只注册函数、类、枚举等顶层符号的“签名”到
    /// 全局作用域。这确保了在分析任何函数体之前，所有顶层符号的名称和类型
    /// 都是已知的。如果在此阶段出现错误（如函数重名），分析会提前终止。
    ///
    /// **第二遍 (Pass 2): 主体分析**
    /// 再次遍历所有顶层声明，这次深入到函数体内部，进行详细的类型检查、
    /// 作用域分析和语义规则验证。
    ///
    /// # Arguments
    ///
    /// * `program` - 一个指向由 `Parser` 生成的 `Program` AST 的引用。
    pub fn analyze(&mut self, program: &Program) {
        // --- 第一遍：注册所有函数签名 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                // NOTE: 此处假设 `register_function_signature` 已被重构为返回 Result<(), SemanticError>
                if let Err(e) = self.register_function_signature(func_decl) {
                    // 将具体的语义错误包装进顶层的 CompilerError 中
                    self.errors.push(CompilerError::Semantic(e));
                }
            }
        }
        
        // 如果在第一遍中就发现了错误（例如，函数重定义），就没有必要继续进行第二遍分析。
        if !self.errors.is_empty() {
            return;
        }

        // --- 第二遍：分析所有函数体 ---
        for toplevel_stmt in &program.body {
            if let TopLevelStatement::Function(func_decl) = toplevel_stmt {
                // NOTE: 此处也假设 `analyze_function_body` 返回 Result<(), SemanticError>
                if let Err(e) = self.analyze_function_body(func_decl) {
                    self.errors.push(CompilerError::Semantic(e));
                }
            }
        }
    }
    
    /// **[第一遍]** 注册一个函数的签名到全局作用域。
    ///
    /// 此函数只关心函数的“外部接口”：它的参数类型和返回类型。
    /// 它会将这些信息组合成一个 `Type::Function`，然后作为一个 `Symbol`
    /// 定义在符号表的全局作用域中。它不会分析函数体内部的任何代码。
    fn register_function_signature(&mut self, func_decl: &FunctionDeclaration) -> Result<(), SemanticError> {
        let mut param_types = Vec::new();
        for p in &func_decl.params {
            // 使用 ? 操作符，如果 string_to_type 失败，错误会立即被传播出去。
            param_types.push(self.string_to_type(&p.param_type)?);
        }
        
        let ret_type = self.string_to_type(&func_decl.return_type)?;
        
        let func_type = Type::Function {
            params: param_types,
            ret: Box::new(ret_type),
        };

        let symbol = Symbol {
            name: func_decl.name.clone(),
            symbol_type: func_type,
            is_mutable: false, // 函数定义本身总是不可变的
        };

        // `self.symbol_table.define` 已经返回 Result<(), SemanticError>，
        // 所以我们可以直接用 ? 来处理可能的“函数重定义”错误。
        self.symbol_table.define(symbol)?;

        Ok(())
    }
    
    /// **[第二遍]** 分析一个函数的函数体。
    ///
    /// 此函数负责深入一个函数的内部，进行详细的语义检查。
    ///
    /// # 执行流程
    /// 1. **进入新作用域**: 为函数体创建一个新的局部作用域。
    /// 2. **设置状态**: 记录下当前函数的返回类型，用于检查 `ret` 语句。
    /// 3. **定义参数**: 将所有函数参数作为变量定义在新创建的局部作用域中。
    /// 4. **分析主体**: 递归地调用语句和表达式的分析函数，检查函数体内的每一行代码。
    /// 5. **离开作用域**: 分析完成后，销毁局部作用域，并清理状态。
    fn analyze_function_body(&mut self, func_decl: &FunctionDeclaration) -> Result<(), SemanticError> {
        // 1. 进入函数作用域
        self.symbol_table.enter_scope();
        
        // 2. 记录当前函数的返回类型
        // 在离开函数时，这个 Option 会被重置为 None
        self.current_return_type = Some(self.string_to_type(&func_decl.return_type)?);

        // 3. 将函数参数定义为新作用域中的变量
        for p in &func_decl.params {
            let param_type = self.string_to_type(&p.param_type)?;
            let param_symbol = Symbol {
                name: p.name.clone(),
                symbol_type: param_type,
                // Tipy 规范中，函数参数默认是不可变的。
                // 未来如果引入 `~` 修饰参数，这里可以修改。
                is_mutable: false,
            };
            self.symbol_table.define(param_symbol)?;
        }
        
        // 4. 分析函数体代码块
        // 我们将在下一步重构 analyze_block_statement
        self.analyze_block_statement(&func_decl.body)?;
        
        // 5. 离开函数作用域并清理状态
        self.symbol_table.leave_scope();
        self.current_return_type = None;

        Ok(())
    }

    // --- 语句与块分析 (Statement & Block Analysis) ---

    /// 分析一个语句，并将其分派给更具体的分析函数。
    ///
    /// 这是语句分析的“路由”，它根据 `Statement` 的不同变体，
    /// 调用相应的处理函数。
    /// # Returns
    /// - `Ok(())` 如果语句及其所有子部分都语义正确。
    /// - `Err(SemanticError)` 如果发现任何语义错误。
    fn analyze_statement(&mut self, statement: &Statement) -> Result<(), SemanticError> {
        // .map(|_| ()) 是一个方便的技巧，用于将 Result<SomeType, E> 转换为 Result<(), E>，
        // 因为在语句上下文中，我们不关心表达式返回的具体类型，只关心它是否出错。
        match statement {
            Statement::VarDeclaration(var_decl) => self.analyze_var_declaration(var_decl),
            Statement::Expression(expression) => self.analyze_expression(expression).map(|_| ()),
            Statement::Return(ret_stmt) => self.analyze_return_statement(ret_stmt),
            Statement::Block(block_stmt) => self.analyze_block_statement(block_stmt).map(|_| ()),
            Statement::While(while_stmt) => self.analyze_while_statement(while_stmt),
            Statement::Break(break_stmt) => self.analyze_break_statement(break_stmt),
            Statement::Continue(cont_stmt) => self.analyze_continue_statement(cont_stmt),
        }
    }

    /// 分析一个代码块 `{ ... }`，并推断出该块的类型。
    ///
    /// # 主要职责
    /// 1. 创建一个新的作用域。
    /// 2. 逐一分析块内的所有语句。
    /// 3. 根据 Tipy 的“隐式返回”规则，确定整个块的类型。
    ///    - 如果块为空，或最后一个语句不是表达式语句，则类型为 `Void`。
    ///    - 否则，类型为最后一个表达式的类型。
    /// 4. 离开作用域。
    fn analyze_block_statement(&mut self, block: &BlockStatement) -> Result<Type, SemanticError> {
        self.symbol_table.enter_scope();
        
        for statement in &block.statements {
            self.analyze_statement(statement)?;
        }

        let block_type = if let Some(last_stmt) = block.statements.last() {
            if let Statement::Expression(expr) = last_stmt {
                self.analyze_expression(expr)?
            } else {
                Type::Void
            }
        } else {
            Type::Void
        };

        self.symbol_table.leave_scope();
        Ok(block_type)
    }

    // --- 具体语句分析(变量声明，函数返回等) ---

    /// 分析变量声明语句 `name: [~]type [= value];`
    fn analyze_var_declaration(&mut self, var_decl: &VarDeclaration) -> Result<(), SemanticError> {
        let var_type = self.string_to_type(&var_decl.var_type)?;

        if let Some(initial_value) = &var_decl.value {
            let value_type = self.analyze_expression(initial_value)?;
            if value_type != var_type {
                // CHANGED: 使用结构化的 TypeMismatch 错误
                return Err(SemanticError::TypeMismatch {
                    expected: var_type,
                    found: value_type,
                    span: Span::default(), // TODO: 从 var_decl 获取 Span
                });
            }
        }

        let symbol = Symbol {
            name: var_decl.name.clone(),
            symbol_type: var_type,
            is_mutable: var_decl.is_mutable,
        };
        
        // .define 已经返回 Result<(), SemanticError>，所以可以直接用 ?
        self.symbol_table.define(symbol)?;
        Ok(())
    }

    /// 分析返回语句 `ret <expression>;`
    fn analyze_return_statement(&mut self, ret_stmt: &ReturnStatement) -> Result<(), SemanticError> {
        // .unwrap_or(Type::Error) 是一个安全的默认值，如果我们在函数外（理论上不可能）
        // 看到了 ret 语句，它会提供一个 Error 类型，避免 panic。
        let expected = self.current_return_type.clone().unwrap_or(Type::Error);

        let actual = match &ret_stmt.value {
            Some(expr) => self.analyze_expression(expr)?,
            None => Type::Void,
        };

        if actual != expected {
            return Err(SemanticError::TypeMismatch {
                expected,
                found: actual,
                span: Span::default(), // TODO: 从 ret_stmt 获取 Span
            });
        }
        Ok(())
    }

    // --- 控制流分析 ---

    /// 分析 `if-elif-else` 表达式，并返回整个表达式的类型。
    fn analyze_if_expression(&mut self, if_expr: &IfExpression) -> Result<Type, SemanticError> {
        let condition_type = self.analyze_expression(&if_expr.condition)?;
        if condition_type != Type::Bool {
            return Err(SemanticError::ConditionNotBoolean { 
                found: condition_type, 
                span: Span::default() // TODO: 从 if_expr.condition 获取 Span
            });
        }

        let consequence_type = self.analyze_block_statement(&if_expr.consequence)?;

        match &if_expr.alternative {
            Some(alt_expr) => {
                let alternative_type = self.analyze_expression(alt_expr)?;
                if consequence_type != alternative_type {
                    return Err(SemanticError::TypeMismatch {
                        expected: consequence_type,
                        found: alternative_type,
                        span: Span::default(), // TODO: 从 alt_expr 获取 Span
                    });
                }
                Ok(consequence_type)
            }
            None => {
                // 根据 Tipy 规范，没有 `else` 的 `if` 是语句，不返回值。
                Ok(Type::Void)
            }
        }
    }

    /// 分析 `loop` 表达式。
    fn analyze_loop_expression(&mut self, loop_expr: &LoopExpression) -> Result<Type, SemanticError> {
        self.loop_depth += 1;
        
        // TODO: 一个更高级的实现会分析所有 `break value` 语句，
        //       并推断出它们的“共同类型”作为 loop 的类型。
        //       目前，我们先简化处理。
        self.analyze_block_statement(&loop_expr.body)?;
        
        self.loop_depth -= 1;
        
        // 暂时假定所有 loop 都返回 void，除非有带值的 break (待实现)。
        Ok(Type::Void)
    }

    /// 分析 `while` 语句。
    fn analyze_while_statement(&mut self, while_stmt: &WhileStatement) -> Result<(), SemanticError> {
        let condition_type = self.analyze_expression(&while_stmt.condition)?;
        if condition_type != Type::Bool {
            return Err(SemanticError::ConditionNotBoolean {
                found: condition_type,
                span: Span::default(), // TODO: 从 while_stmt.condition 获取 Span
            });
        }

        self.loop_depth += 1;
        // `while` 循环是语句，不返回值，所以我们忽略 `analyze_block_statement` 的结果。
        self.analyze_block_statement(&while_stmt.body)?;
        self.loop_depth -= 1;

        Ok(())
    }

    /// 分析 `break` 语句。
    fn analyze_break_statement(&mut self, _break_stmt: &BreakStatement) -> Result<(), SemanticError> {
        if self.loop_depth == 0 {
            return Err(SemanticError::IllegalBreak { span: Span::default() }); // TODO: 从 _break_stmt 获取 Span
        }
        // TODO: 分析 _break_stmt.value 的类型，并与当前循环的期望返回类型比较。
        Ok(())
    }

    /// 分析 `continue` 语句。
    fn analyze_continue_statement(&mut self, _cont_stmt: &ContinueStatement) -> Result<(), SemanticError> {
        if self.loop_depth == 0 {
            return Err(SemanticError::IllegalContinue { span: Span::default() }); // TODO: 从 _cont_stmt 获取 Span
        }
        Ok(())
    }
    
    // --- 表达式分析 (Expression Analysis) ---

    /// 分析一个表达式，并递归地推断出它的类型。
    ///
    /// 这是语义分析的核心递归函数。它使用 `match` 将不同种类的表达式
    /// 分派给各自的、更具体的分析辅助函数。
    ///
    /// # Returns
    /// - `Ok(Type)` 如果表达式及其所有子表达式都语义正确。
    /// - `Err(SemanticError)` 如果发现任何类型错误、未定义符号等问题。
    fn analyze_expression(&mut self, expression: &Expression) -> Result<Type, SemanticError> {
        match expression {
            Expression::Literal(lit) => self.analyze_literal_expression(lit),
            Expression::Identifier(name) => self.analyze_identifier_expression(name),
            Expression::Assignment(assign_expr) => self.analyze_assignment_expression(assign_expr),
            Expression::Prefix(prefix_expr) => self.analyze_prefix_expression(prefix_expr),
            Expression::Infix(infix_expr) => self.analyze_infix_expression(infix_expr),
            Expression::Call(call_expr) => self.analyze_call_expression(call_expr),
            Expression::If(if_expr) => self.analyze_if_expression(if_expr),
            Expression::Loop(loop_expr) => self.analyze_loop_expression(loop_expr),
            Expression::Block(block_stmt) => self.analyze_block_statement(block_stmt),
        }
    }

    // --- 表达式分析辅助函数 (Expression Analysis Helpers) ---

    fn analyze_literal_expression(&self, lit: &Literal) -> Result<Type, SemanticError> {
        // 根据字面量的种类，直接返回其对应的内部类型。
        // 这是类型推断递归的基准情形 (base case)。
        match lit {
            Literal::Integer(_) => Ok(Type::I64), // TODO: 根据字面量后缀（如 10u8）推断更精确的整数类型
            Literal::Float(_) => Ok(Type::F64),   // TODO: 支持 f32
            Literal::Boolean(_) => Ok(Type::Bool),
            Literal::Char(_) => Ok(Type::Char),
            Literal::String(_) => Ok(Type::Str),
        }
    }

    fn analyze_identifier_expression(&self, name: &str) -> Result<Type, SemanticError> {
        // 对于一个标识符，它的类型就是它在符号表中记录的类型。
        if let Some(symbol) = self.symbol_table.lookup(name) {
            Ok(symbol.symbol_type.clone())
        } else {
            // 如果在符号表中找不到，说明该变量或函数未被定义。
            Err(SemanticError::SymbolNotFound {
                name: name.to_string(),
                span: Span::default(), // TODO: 从 Expression 节点获取 Span
            })
        }
    }

    fn analyze_assignment_expression(&mut self, assign_expr: &AssignmentExpression) -> Result<Type, SemanticError> {
        // 分析赋值表达式 e.g., `x = 10`
        let value_type = self.analyze_expression(&assign_expr.value)?;

        // 检查赋值目标（左值 L-Value）
        // 目前，我们只支持对简单标识符的赋值。
        if let Expression::Identifier(name) = &*assign_expr.left {
            let symbol = match self.symbol_table.lookup(name) {
                Some(s) => s,
                None => return Err(SemanticError::SymbolNotFound {
                    name: name.clone(),
                    span: Span::default(), // TODO: Span
                }),
            };

            if !symbol.is_mutable {
                // 如果变量不是用 `~` 声明的，则不允许赋值。
                // return Err(...) // TODO: 添加 `CannotAssignToImmutable` 错误
            }

            if symbol.symbol_type != value_type {
                return Err(SemanticError::TypeMismatch {
                    expected: symbol.symbol_type.clone(),
                    found: value_type,
                    span: Span::default(), // TODO: Span
                });
            }

            // 赋值表达式本身的类型就是被赋的值的类型。
            Ok(value_type)
        } else {
            // 如果赋值目标不是一个标识符（例如 `5 = 10`），则为非法赋值。
            Err(SemanticError::InvalidAssignmentTarget { span: Span::default() }) // TODO: Span
        }
    }
    
    fn analyze_prefix_expression(&mut self, prefix_expr: &PrefixExpression) -> Result<Type, SemanticError> {
        let right_type = self.analyze_expression(&prefix_expr.right)?;
        
        match prefix_expr.op {
            PrefixOperator::Minus => match right_type {
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Isize |
                Type::F32 | Type::F64 => Ok(right_type), // 负号不改变数字类型
                _ => {
                    // FIXED: 使用我们新的、更具体的错误类型
                    Err(SemanticError::InvalidOperatorForType {
                        operator: "-".to_string(),
                        the_type: right_type,
                        span: Span::default(), // TODO: 从 prefix_expr 获取 Span
                    })
                }
            },
            PrefixOperator::Not => {
                if right_type == Type::Bool {
                    Ok(Type::Bool) // `!` 作用于布尔值，结果仍是布尔值
                } else {
                    // FIXED: 完整地构造错误
                    Err(SemanticError::InvalidOperatorForType {
                        operator: "!".to_string(),
                        the_type: right_type,
                        span: Span::default(), // TODO: 从 prefix_expr 获取 Span
                    })
                }
            }
        }
    }

    fn analyze_infix_expression(&mut self, infix_expr: &InfixExpression) -> Result<Type, SemanticError> {
        let left_type = self.analyze_expression(&infix_expr.left)?;
        let right_type = self.analyze_expression(&infix_expr.right)?;

        // TODO: 更复杂的类型规则，例如 i32 + f64 的类型提升
        if left_type != right_type {
            return Err(SemanticError::TypeMismatch { expected: left_type, found: right_type, span: Span::default() });
        }

        match infix_expr.op {
            // 算术运算返回原类型
            Operator::Plus | Operator::Minus | Operator::Multiply | Operator::Divide => {
                // 确保操作数是数字类型
                Ok(left_type)
            },
            // 比较运算总是返回布尔类型
            Operator::Equal | Operator::NotEqual | Operator::LessThan |
            Operator::LessEqual | Operator::GreaterThan | Operator::GreaterEqual => {
                Ok(Type::Bool)
            },
        }
    }

    fn analyze_call_expression(&mut self, call_expr: &CallExpression) -> Result<Type, SemanticError> {
        let callee_type = self.analyze_expression(&call_expr.function)?;
        
        match callee_type {
            Type::Function { params: expected_params, ret: ret_type } => {
                // 1. 检查参数数量
                if call_expr.arguments.len() != expected_params.len() {
                    return Err(SemanticError::ArityMismatch {
                        expected: expected_params.len(),
                        found: call_expr.arguments.len(),
                        span: Span::default(), // TODO: Span
                    });
                }
                // 2. 检查每个参数的类型
                for (arg_expr, expected_type) in call_expr.arguments.iter().zip(expected_params.iter()) {
                    let arg_type = self.analyze_expression(arg_expr)?;
                    if arg_type != *expected_type {
                        return Err(SemanticError::TypeMismatch {
                            expected: expected_type.clone(),
                            found: arg_type,
                            span: Span::default(), // TODO: Span
                        });
                    }
                }
                // 3. 所有检查通过，返回函数的返回类型
                Ok(*ret_type)
            },
            other_type => Err(SemanticError::NotAFunction {
                found: other_type,
                span: Span::default(), // TODO: Span
            }),
        }
    }

    /// 将 AST 中的类型字符串（如 "i32", "^~bool"）解析为内部的 `Type` 枚举。
    ///
    /// 这是类型解析的核心。它能够处理原生类型、指针类型等。
    ///
    /// # Arguments
    /// * `type_str` - 从 AST 节点（如 `VarDeclaration`）中获取的类型字符串。
    ///
    /// # Returns
    /// - `Ok(Type)` 如果字符串是一个合法的、已知的类型。
    /// - `Err(SemanticError)` 如果类型名称未知。
    fn string_to_type(&self, type_str: &str) -> Result<Type, SemanticError> {
        // TODO: 这是一个简化的实现。一个完整的实现会更健壮，
        //       并且能够解析用户自定义的类型（如类名）。
        //       目前，我们先支持原生类型和指针。
        
        // 暂时简单地根据字符串匹配返回类型
        match type_str {
            "i8" => Ok(Type::I8),
            "i16" => Ok(Type::I16),
            "i32" => Ok(Type::I32),
            "i64" => Ok(Type::I64),
            "f32" => Ok(Type::F32),
            "f64" => Ok(Type::F64),
            "bool" => Ok(Type::Bool),
            "char" => Ok(Type::Char),
            "str" => Ok(Type::Str),
            "void" => Ok(Type::Void),
            _ => {
                // 如果不是已知原生类型，我们返回一个“未找到符号”的错误。
                // 因为一个未知的类型名，本质上就是一个未定义的类型符号。
                Err(SemanticError::SymbolNotFound {
                    name: type_str.to_string(),
                    // TODO: 这里需要一个真实的 Span
                    span: Span::default(),
                })
            }
        }
    }
}



