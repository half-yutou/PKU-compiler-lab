# PKU Compiler Lab
北大编译原理实验

## 自测运行
> 请先根据实验文档配置本地docker环境  
```shell

# 本地运行命令
cargo run -- -koopa hello.c -o koopair.txt
cargo run -- -riscv hello.c -o riscv.txt

# 本地测试命令
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest -koopa -s lv${LEVEL} /root/compiler
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest -riscv -s lv${LEVEL} /root/compiler

# 全case测试(此分支lab9+会失败)
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest -koopa /root/compiler
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest -riscv /root/compiler
```

## 项目结构

parser工具:https://github.com/lalrpop/lalrpop  

```c
├── Cargo.toml
├── README.md
├── build.rs    // lalrpop构建脚本
├── hello.c     // 测试编译的SysY文件
└── src         // 编译器源码
    ├── ast.rs  // 抽象语法树
    ├── lab8    
    │         ├── codegen.rs // 目标代码(riscv)生成模块
    │         ├── codegen    
    │         ├── irgen.rs   // 中间代码(koopa)生成模块
    │         ├── irgen      
    │         │         ├── block.rs     // 语句块
    │         │         ├── calc.rs      // 表达式计算
    │         │         ├── declare.rs   // 声明与定义语句
    │         │         ├── statement.rs // Stmt语句
    │         │         ├── symbol.rs    // 符号表
    │         │         └── vars.rs      // 变量计算
    │         └── mod.rs
    ├── lib.rs
    ├── main.rs
    └── sysy.lalrpop // parser
```

## AST(抽象语法树)的设计

以当前实现的rust代码为基准的ast表示形式  
(完整的EBNF形式请见实验指导文档)  

```ast
CompUnit {
    items: Vec<CompUnitItem( 
        FuncDef {
            func_type: FuncType(Void, Int), 
            id: String, 
            params: Option<FuncFParams {
                params: Vec<FuncFParam {
                    b_type: String, 
                    ident: String, 
                }>
            }>, 
            block: Block {
                block_item_list: Vec<BlockItem(
                    Decl(
                        ConstDecl {
                            b_type: String, 
                            const_def_list: Vec<ConstDef {
                                ident: String, 
                                const_init_val: ConstInitVal {
                                    const_exp: ConstExp {
                                        lor_exp: LOrExp(...)
                                    }, 
                                },
                            }>
                        }, 
                        VarDecl {
                            b_type: String, 
                            var_def_list: Vec<VarDef {
                                ident: String, 
                                init_val: Option<InitVal {
                                    exp: Exp(Lor(Box<LOrExp(...)>))
                                }>
                            }>
                        }, 
                    ), 
                    Stmt(
                        Exp(Option<Exp>), 
                        Return(Option<Exp>),
                        Assign(LVal {ident: String}, Exp),
                        If(Exp, Box<Stmt>, Option<Box<Stmt>>), 
                        While(Exp, Box<Stmt>), 
                        Break, 
                        Continue, 
                        Block<Block>  
                    )
                )>
            }, 
        }, 
        
        GlobalDecl(
            ConstDecl {
                b_type: String, 
                const_def_list: Vec<ConstDef {
                    ident: String, 
                    const_init_val: ConstInitVal {
                        const_exp: ConstExp {
                            lor_exp: LOrExp(...)
                        }, 
                    }
                }>
            },
            GlobalVarDecl {
                b_type: String, 
                var_def_list: Vec<GlobalVarDef {
                    ident: String, 
                    init_val: Option<InitVal {
                        exp: Exp(Lor(Box<LorExp(...)>))
                    }>
                }>
            }, 
        ), 
    )> 
}

// 表达式（优先级从上至下-配合后续遍历达到控制优先级的效果）
Exp (最低优先级)
└── LOrExp (逻辑或 ||)
     └── LAndExp (逻辑与 &&)
         └── EqExp (相等比较 == !=)
             └── RelExp (关系比较 < > <= >=)
                 └── AddExp (加减 + -)
                     └── MulExp (乘除模 * / %)
                         └── UnaryExp (一元运算( + - ! ) 与 函数调用(FuncCall))
                             └── PrimaryExp (i32, Paren(括号表达式), LVal(左值))
```


## 嵌套作用域符号表

在lab5之前，我们实现了一个支持单作用域的符号表  
其实现思路也很简单，就是使用一个HashMap结构记录变量名和变量值  
```rust
type SymbolTable = HashMap<String, i32>;
```

但是在lab5中，我们需要支持如下的嵌套作用域  
```c
int main() {
    int a = 10;
    int b = 12;
    {
        int a = 11;
        return a; // a == 11, b == 12
    }
    return 0;
}
```
可以看到，内层作用域可以重新定义同名外界变量，且将外界同名变量遮蔽  
也就是说改造后的符号表需要支持：  
1. 进入新作用域后隔离记录变量信息
2. 内层变量信息遮蔽外层同名变量信息
3. 内层可以访问到外层作用域变量  
4. 退出作用域后销毁当前作用域变量信息
  
故我们容易想到使用**栈**这种**后进先出**的数据结构改造符号表
```rust
pub struct ScopeStack {
    scopes: Vec<HashMap<String, SymbolInfo>>,  // 作用域栈，每层是一个符号表
    var_counter: HashMap<String, usize>,       // 变量重命名计数器
}
```

并为其实现以下方法
```rust
impl ScopeStack {
    pub fn new() -> Self {} // 初始化，此时塞入第一个HashMap代表全局作用域

    pub fn enter_scope(&mut self) {} // 进入新的作用域，压栈一个HashMap

    pub fn exit_scope(&mut self) {} // 退出作用域，出栈一个HashMap

    pub fn define(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {} // 在当前定义域新增符号

    pub fn lookup(&self, name: &str) -> Option<&SymbolInfo> {} // 从内层向外层依次寻找变量值
}
```

这样的一个符号表结构便能完美隔离作用域  

## 处理if跳转

如果你和本实现一样，使用`then,else,end`三段式处理if-else语句的跳转，    
然后使用`jump`,`br`这样的koopaIR命令对程序块进行流程控制后，也许会发现总有一些测试用例无法通过  
你可以检查生成的koopaIR是否在`end`语句块没有终结指令(`ret`, `jump`, `br`)  
我们注意到koopaIR有这么一些规则(来自lab6实验指导书):
> 需要注意的是,   
> 基本块的结尾必须是 br, jump 或 ret 指令其中之一 (并且, 这些指令只能出现在基本块的结尾).  
>   
> 也就是说, 即使两个基本块是相邻的,  
> 如果你想表达执行完前者之后执行后者的语义, 你也必须在前者基本块的结尾添加一条目标为后者的 jump 指令.  
> 这点和汇编语言中 label 的概念有所不同.  

我们以一个实际的例子来看看可能会出现的错误: (嵌套if块)
```c
int main() {
    if (A) {
        if (B) {
            S1;
        } // <-- then 块在这里自然结束了，进入end2
    }
    // <- end1
    return 0;
}

```
在上面这个例子中，需要额外记录内层end2跳转转外层end1，  
否则end2便是空块，不满足上述koopaIR对语句块的要求。  

这时有一个符合直觉的思路，那么就是在end2进行时直接记录跳转end1语句  
但是这时又会有一个问题，如果end2部分出现了提前`return`，那么此块就会同时有两个终结指令  

为了解决问题，有一个“糟糕”但有效的办法：    
我们仅在处理if语句时记录其end块对应的外层块，但是不额外添加终结指令  
当所以指令都添加完毕后，再遍历每个块判断是否有终结指令，如果其不含终结指令就为其添加一条向记录中的语句块的跳转  

这也就是如下几个方法的作用
```rust
impl IRGen {
    pub fn push_control_flow(&mut self, end_bb: BasicBlock, context_type: ControlFlowType) {}

    pub fn pop_control_flow(&mut self) -> Option<ControlFlowContext> {}

    pub fn record_pending_jump(&mut self, current_end_bb: BasicBlock) {}

    pub fn process_pending_jumps(&mut self) {}
}
```


## 短路求值

由于koopaIR仅支持**按位与或**，在lab3对表达式的处理中  
我们使用以下方式进行**逻辑与或**向**按位与或**的转换   
```rust
/// koopaIR 只支持按位或，如何实现逻辑或？
/// ```txt
/// (left || right) <=> (left != 0) | (right != 0)
/// ```

/// koopaIR只支持按位与，如何实现逻辑与
/// ```txt
/// (left && right) <=> (left != 0) & (right != 0)
/// ```
```

在这个实现中我们同时对左右表达式进行了计算，明显不符合短路运算的要求  
在完成了if语句部门，掌握了控制流后，我们便可以做到使用控制流和语句块分别计算左右语句
```c
if (left || right) {}

// 等价于
if (left) {}
else if (right) {}
else {}

// ==========

if (left && right) {}

// 等价于

if (left) {
    if (right) {
    
    }
}
```










