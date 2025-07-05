; ModuleID = 'tipy_module'
source_filename = "tipy_module"

@.str = private unnamed_addr constant [19 x i8] c"Hello, Tipy World!\00", align 1

declare i32 @printf(ptr, ...)

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @.str)
  ret i32 0
}
