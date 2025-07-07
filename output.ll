; ModuleID = 'tipy_module'
source_filename = "tipy_module"

define i32 @add(i32 %a, i32 %b) {
entry:
  %b2 = alloca i32, align 4
  %a1 = alloca i32, align 4
  store i32 %a, ptr %a1, align 4
  store i32 %b, ptr %b2, align 4
  %a3 = load i32, ptr %a1, align 4
  %b4 = load i32, ptr %b2, align 4
  %tmpadd = add i32 %a3, %b4
  ret i32 %tmpadd
}

define i32 @main() {
entry:
  %b = alloca i32, align 4
  %a = alloca i32, align 4
  store i32 10, ptr %a, align 4
  store i32 20, ptr %b, align 4
  %a1 = load i32, ptr %a, align 4
  %b2 = load i32, ptr %b, align 4
  %tmpmul = mul i32 %b2, 2
  %tmpcall = call i32 @add(i32 %a1, i32 %tmpmul)
  store i32 %tmpcall, ptr %b, align 4
  %b3 = load i32, ptr %b, align 4
  ret i32 %b3
}

declare i32 @puts(ptr)
