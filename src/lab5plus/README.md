# lab5 summary

## 这个模块是干什么的？

将直至lab5中的代码“大杂烩”整理，拆分为以下几个部分：
> 注：本模块采用了非`mod.rs`模式进行模块拆分

- irgen:   中间代码生成(koopaIR)
    - symbol: 符号表
    - block:  IR生成逻辑入口
      - declare:   常量变量定义
        - calc: 运算表达式，常量值计算
        - vars: 变量定义与计算
      - statement: 语句(赋值语句，返回语句，副作用语句，嵌套代码块)
        - vars: 变量定义与计算

- codegen: 目标代码生成(riscv)