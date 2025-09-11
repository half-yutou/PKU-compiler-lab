//! ```text
//! Exp (最低优先级)
//! └── LOrExp      (逻辑或 ||)
//!     └── LAndExp  (逻辑与 &&)
//!         └── EqExp    (相等比较 == !=)
//!             └── RelExp   (关系比较 < > <= >=)
//!                 └── AddExp   (加减 + -)
//!                     └── MulExp   (乘除模 * / %)
//!                         └── UnaryExp (一元运算 + - !)
//!                             └── PrimaryExp (最高优先级)
//! ```

use crate::ast::{AddExp, EqExp, EqOp, Exp, LAndExp, LOrExp, LVal, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, UnaryExp, UnaryOp};
use crate::lab9::irgen::symbol::SymbolInfo;
use crate::lab9::irgen::IRGen;
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use koopa::ir::{BinaryOp, Type, Value};

impl IRGen {
    pub fn generate_exp(&mut self, exp: &Exp) -> Value {
        match exp { 
            Exp::LOr(lor_exp) => self.generate_lor_exp(lor_exp)
        }
    }

    /// 生成lor表达式的ir - 实现短路求值
    fn generate_lor_exp(&mut self, lor_exp: &LOrExp) -> Value {
        match lor_exp {
            LOrExp::LAnd(land_exp) => self.generate_land_exp(land_exp),
            LOrExp::LOr(left, right) => {
                // 关键：传递 AST 节点而不是提前计算的值
                self.generate_lor_binary_op_ast(left, right)
            }
        }
    }

    /// 生成land表达式的ir - 实现短路求值
    fn generate_land_exp(&mut self, land_exp: &LAndExp) -> Value {
        match land_exp {
            LAndExp::Eq(eq_exp) => self.generate_eq_exp(eq_exp),
            LAndExp::LAnd(left, right) => {
                // 关键：传递 AST 节点而不是提前计算的值
                self.generate_land_binary_op_ast(left, right)
            }
        }
    }

    /// 接收 AST 节点，实现短路求值
    fn generate_lor_binary_op_ast(&mut self, left: &LOrExp, right: &LAndExp) -> Value {
        self.function_irgen.bb_counter += 1;
        
        // 先创建所有需要的基本块
        let eval_rhs;
        let result_true;
        let result_false;
        let lor_end;

        {
            let bb_counter = self.function_irgen.bb_counter;
            let func_data = self.function_data_mut();
            eval_rhs = func_data.dfg_mut().new_bb().basic_block(Some(format!("%eval_rhs_{}", bb_counter)));
            result_true = func_data.dfg_mut().new_bb().basic_block(Some(format!("%result_true_{}", bb_counter)));
            result_false = func_data.dfg_mut().new_bb().basic_block(Some(format!("%result_false_{}", bb_counter)));
            lor_end = func_data.dfg_mut().new_bb()
                .basic_block_with_params(Some(format!("%lor_end_{}", bb_counter)), vec![Type::get_i32()]);
        }
        
        // 添加基本块到函数布局
        {
            let func_data = self.function_data_mut();
            func_data.layout_mut().bbs_mut().push_key_back(eval_rhs).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(result_true).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(result_false).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(lor_end).unwrap();
        }
        
        let left_value = self.generate_lor_exp(left);
        
        // 检查左操作数是否为真
        let left_cond;
        let branch;
        {
            let current_bb = self.current_bb();
            let func_data = self.function_data_mut();
            let zero = func_data.dfg_mut().new_value().integer(0);
            left_cond = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left_value, zero);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(left_cond).unwrap();
            
            // 短路判断：如果左操作数为真，直接跳到 result_true；否则跳到 eval_rhs
            branch = func_data.dfg_mut().new_value().branch(left_cond, result_true, eval_rhs);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(branch).unwrap();
        }
        
        // result_true 基本块：左操作数为真，结果为 1
        {
            self.function_irgen.current_bb = Some(result_true);
            let func_data = self.function_data_mut();
            let true_value = func_data.dfg_mut().new_value().integer(1);
            let jump_true = func_data.dfg_mut().new_value().jump_with_args(lor_end, vec![true_value]);
            func_data.layout_mut().bb_mut(result_true).insts_mut().push_key_back(jump_true).unwrap();
        }
        
        // eval_rhs 基本块：只有在左操作数为假时才计算右操作数
        self.function_irgen.current_bb = Some(eval_rhs);
        let right_value = self.generate_land_exp(right);
        
        let right_cond;
        let branch_rhs;
        {
            let current_bb = self.current_bb();
            
            let func_data = self.function_data_mut();
            let zero_rhs = func_data.dfg_mut().new_value().integer(0);
            right_cond = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right_value, zero_rhs);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(right_cond).unwrap();
            
            // 根据右操作数的值跳转
            branch_rhs = func_data.dfg_mut().new_value().branch(right_cond, result_true, result_false);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(branch_rhs).unwrap();
        }
        
        // result_false 基本块：右操作数也为假，结果为 0
        {
            self.function_irgen.current_bb = Some(result_false);
            let func_data = self.function_data_mut();
            let false_value = func_data.dfg_mut().new_value().integer(0);
            let jump_false = func_data.dfg_mut().new_value().jump_with_args(lor_end, vec![false_value]);
            func_data.layout_mut().bb_mut(result_false).insts_mut().push_key_back(jump_false).unwrap();
        }
        
        // 更新当前基本块为 lor_end
        self.function_irgen.current_bb = Some(lor_end);
        
        // 获取基本块参数作为结果（相当于 phi 节点的结果）
        let func_data = self.function_data_mut();
        let phi_result = func_data.dfg().bb(lor_end).params()[0];
        phi_result
    }

    /// 接收 AST 节点，实现短路求值
    fn generate_land_binary_op_ast(&mut self, left: &LAndExp, right: &EqExp) -> Value {
        self.function_irgen.bb_counter += 1;
        
        // 先创建所有需要的基本块
        let eval_rhs;
        let result_true;
        let result_false;
        let land_end;

        {
            let bb_counter = self.function_irgen.bb_counter;
            let func_data = self.function_data_mut();
            eval_rhs = func_data.dfg_mut().new_bb().basic_block(Some(format!("%eval_rhs_{}", bb_counter)));
            result_true = func_data.dfg_mut().new_bb().basic_block(Some(format!("%result_true_{}", bb_counter)));
            result_false = func_data.dfg_mut().new_bb().basic_block(Some(format!("%result_false_{}", bb_counter)));
            land_end = func_data.dfg_mut().new_bb()
                .basic_block_with_params(Some(format!("%land_end_{}", bb_counter)), vec![Type::get_i32()]);
        }
        
        // 添加基本块到函数布局
        {
            let func_data = self.function_data_mut();
            func_data.layout_mut().bbs_mut().push_key_back(eval_rhs).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(result_true).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(result_false).unwrap();
            func_data.layout_mut().bbs_mut().push_key_back(land_end).unwrap();
        }
        
        // 在当前基本块中计算左操作数
        // 如果左操作数的计算改变了当前基本块，我们需要使用新的当前基本块
        let left_value = self.generate_land_exp(left);
        
        // 检查左操作数是否为假
        let zero;
        let left_cond;
        let branch;
        {
            let current_bb = self.current_bb();
            let func_data = self.function_data_mut();
            zero = func_data.dfg_mut().new_value().integer(0);
            left_cond = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left_value, zero);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(left_cond).unwrap();
            
            // 短路判断：如果左操作数为假，直接跳到 result_false；否则跳到 eval_rhs
            branch = func_data.dfg_mut().new_value().branch(left_cond, eval_rhs, result_false);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(branch).unwrap();
        }
        
        // result_false 基本块：左操作数为假，结果为 0
        {
            self.function_irgen.current_bb = Some(result_false);
            let func_data = self.function_data_mut();
            let false_value = func_data.dfg_mut().new_value().integer(0);
            let jump_false = func_data.dfg_mut().new_value().jump_with_args(land_end, vec![false_value]);
            func_data.layout_mut().bb_mut(result_false).insts_mut().push_key_back(jump_false).unwrap();
        }
        
        // eval_rhs 基本块：只有在左操作数为真时才计算右操作数
        self.function_irgen.current_bb = Some(eval_rhs);
        let right_value = self.generate_eq_exp(right);
        
        let right_cond;
        let branch_rhs;
        {
            let current_bb = self.current_bb();
            let func_data = self.function_data_mut();
            let zero_rhs = func_data.dfg_mut().new_value().integer(0);
            right_cond = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right_value, zero_rhs);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(right_cond).unwrap();
            
            // 根据右操作数的值跳转
            branch_rhs = func_data.dfg_mut().new_value().branch(right_cond, result_true, result_false);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(branch_rhs).unwrap();
        }
        
        // result_true 基本块：右操作数为真，结果为 1
        {
            self.function_irgen.current_bb = Some(result_true);
            let func_data = self.function_data_mut();
            let true_value = func_data.dfg_mut().new_value().integer(1);
            let jump_true = func_data.dfg_mut().new_value().jump_with_args(land_end, vec![true_value]);
            func_data.layout_mut().bb_mut(result_true).insts_mut().push_key_back(jump_true).unwrap();
        }
        
        // 更新当前基本块为 land_end
        self.function_irgen.current_bb = Some(land_end);
        
        // 获取基本块参数作为结果
        let func_data = self.function_data_mut();
        let phi_result = func_data.dfg().bb(land_end).params()[0];
        phi_result
    }

    /// 生成eq表达式的ir
    fn generate_eq_exp(&mut self, eq_exp: &EqExp) -> Value {
        match eq_exp {
            EqExp::Rel(rel_exp) => self.generate_rel_exp(rel_exp),
            EqExp::Eq(left, op, right) => {
                let left_value = self.generate_eq_exp(left);
                let right_value = self.generate_rel_exp(right);
                self.generate_eq_binary_op(op, left_value, right_value)
            }
        }
    }

    fn generate_eq_binary_op(&mut self, op: &EqOp, left: Value, right: Value) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        let binary_op = match op {
            EqOp::Eq => BinaryOp::Eq,
            EqOp::Ne => BinaryOp::NotEq,
        };

        let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(inst).unwrap();
        inst
    }

    /// 生成rel表达式的ir
    fn generate_rel_exp(&mut self, rel_exp: &RelExp) -> Value {
        match rel_exp {
            RelExp::Add(add_exp) => self.generate_add_exp(add_exp),
            RelExp::Rel(left, op, right) => {
                let left_value = self.generate_rel_exp(left);
                let right_value = self.generate_add_exp(right);
                self.generate_rel_binary_op(op, left_value, right_value)
            }
        }
    }

    fn generate_rel_binary_op(&mut self, op: &RelOp, left: Value, right: Value) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        let binary_op = match op {
            RelOp::Lt => BinaryOp::Lt,
            RelOp::Gt => BinaryOp::Gt,
            RelOp::Le => BinaryOp::Le,
            RelOp::Ge => BinaryOp::Ge,
        };

        let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(inst).unwrap();
        inst
    }

    /// 生成add表达式的ir
    fn generate_add_exp(&mut self, add_exp: &AddExp) -> Value {
        match add_exp {
            AddExp::Mul(mul_exp) => self.generate_mul_exp(mul_exp),
            AddExp::AddMul(left, op, right) => {
                let left_value = self.generate_add_exp(left);
                let right_value = self.generate_mul_exp(right);
                self.generate_add_binary_op(op, left_value, right_value)
            }
        }
    }

    fn generate_add_binary_op(&mut self, op: &PlusSubOp, left: Value, right: Value) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        let binary_op = match op {
            PlusSubOp::Plus => BinaryOp::Add,
            PlusSubOp::Minus => BinaryOp::Sub,
        };

        let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(inst).unwrap();
        inst
    }

    /// 生成mul表达式的ir
    fn generate_mul_exp(&mut self, mul_exp: &MulExp) -> Value {
        match mul_exp {
            MulExp::Unary(unary_exp) => self.generate_unary_exp(unary_exp),
            MulExp::MulDiv(left, op, right) => {
                let left_value = self.generate_mul_exp(left);
                let right_value = self.generate_unary_exp(right);
                self.generate_mul_binary_op(op, left_value, right_value)
            }
        }
    }

    fn generate_mul_binary_op(&mut self, op: &MulDivOp, left: Value, right: Value) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        let binary_op = match op {
            MulDivOp::Mul => BinaryOp::Mul,
            MulDivOp::Div => BinaryOp::Div,
            MulDivOp::Mod => BinaryOp::Mod,
        };

        let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(inst).unwrap();
        inst
    }

    /// 生成unary表达式(一元表达式)的ir
    fn generate_unary_exp(&mut self, unary_exp: &UnaryExp) -> Value {
        match unary_exp {
            UnaryExp::Primary(primary) => self.generate_primary_exp(primary),
            UnaryExp::Unary(op, exp) => {
                let operand = self.generate_unary_exp(exp);
                self.generate_unary_op(op, operand)
            }
            UnaryExp::FuncCall(func_name, params) => {
                // 查找函数句柄
                let function_handler = if let Some(&func_handler) = self.functions.get(func_name) {
                    func_handler
                } else {
                    panic!("Function '{}' not found", func_name);
                };
                
                // 生成参数列表的 IR 值
                let mut args = Vec::new();
                if let Some(func_params) = params {
                    for param_exp in &func_params.params {
                        let arg_value = self.generate_arg_exp(param_exp);
                        args.push(arg_value);
                    }
                }
                
                // 生成函数调用指令
                 let current_bb = self.current_bb();
                 let func_data = self.function_data_mut();
                 
                 // 调试输出：打印函数名和参数类型
                 eprintln!("DEBUG: Calling function '{}' with {} args", func_name, args.len());
                 for (i, &arg) in args.iter().enumerate() {
                     let arg_type = func_data.dfg().value(arg).ty();
                     eprintln!("DEBUG: Arg {}: {:?}", i, arg_type);
                 }
                 
                 let call_inst = func_data.dfg_mut().new_value().call(function_handler, args);
                 func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(call_inst).unwrap();
                
                call_inst
            }
        }
    }

    fn generate_unary_op(&mut self, op: &UnaryOp, operand: Value) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        match op {
            UnaryOp::Plus => operand,
            UnaryOp::Minus => {
                let zero = func_data.dfg_mut().new_value().integer(0);
                let sub_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Sub, zero, operand);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(sub_inst).unwrap();
                sub_inst
            },
            UnaryOp::Not => {
                let zero = func_data.dfg_mut().new_value().integer(0);
                let eq_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Eq, operand, zero);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(eq_inst).unwrap();
                eq_inst
            }
        }
    }

    fn generate_primary_exp(&mut self, primary: &PrimaryExp) -> Value {
        match primary {
            PrimaryExp::Number(num) => {
                let func_data = self.function_data_mut();
                func_data.dfg_mut().new_value().integer(*num)
            },
            PrimaryExp::Paren(exp) => self.generate_exp(exp),
            PrimaryExp::LVal(lval) => self.generate_lval_load(lval),
        }
    }
    
    // 左值被调用时，返回其对应值的ptr
    pub fn generate_lval_load(&mut self, lval: &LVal) -> Value {
        let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();
        
        match symbol_info {
            Some(SymbolInfo::Const(value)) => {
                if !lval.indices.is_empty() {
                    panic!("Cannot index into scalar constant");
                }
                let func_data = self.function_data_mut();
                func_data.dfg_mut().new_value().integer(value)
            }
            Some(SymbolInfo::Var(ptr)) | Some(SymbolInfo::GlobalVar(ptr)) => {
                if !lval.indices.is_empty() {
                    panic!("Cannot index into scalar variable");
                }
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let load_inst = func_data.dfg_mut().new_value().load(ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(load_inst).unwrap();
                load_inst
            }

            // region 数组访问
            Some(SymbolInfo::LocalConstArray(ptr, _)) | 
            Some(SymbolInfo::GlobalConstArray(ptr, _)) |
            Some(SymbolInfo::LocalArray(ptr, _)) |
            Some(SymbolInfo::GlobalArray(ptr, _)) => {
                if lval.indices.is_empty() {
                    panic!("Cannot load entire array '{}'", lval.ident);
                }
                
                // 变量数组元素访问处理
                let elem_ptr = self.generate_array_access_ptr(ptr, &lval.indices);
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let load_inst = func_data.dfg_mut().new_value().load(elem_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(load_inst).unwrap();
                load_inst
            }

            // 数组参数访问：使用 getptr 和 getelemptr 组合
            Some(SymbolInfo::ParamArray(param_ptr, _)) => {
                if lval.indices.is_empty() {
                    panic!("Cannot load entire parameter array '{}'", lval.ident);
                }

                let param_type = self.function_data_mut().dfg().value(param_ptr).ty().clone();
                println!("Debug: visiting param_type is {:?}", param_type);
                
                // 先计算所有索引
                let indexes: Vec<Value> = lval.indices
                    .iter()
                    .map(|exp| self.generate_exp(&exp))
                    .collect();
                
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();

                // 解引用局部变量，得到真正的指针**i32 -> *i32
                let loaded_ptr = func_data.dfg_mut().new_value().load(param_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(loaded_ptr).unwrap();
                
                // 从参数指针开始逐层解开
                let mut current_ptr = loaded_ptr;
                
                for (i, &index) in indexes.iter().enumerate() {
                    let param_type = func_data.dfg().value(current_ptr).ty().clone();
                    println!("Debug: i = {}, current_ptr_type = {}", i, param_type);
                    if i == 0 {
                        // 第一层：对指针类型使用 getptr
                        // loaded_ptr 类型是 *[i32, 3], 使用 getptr 进行指针算术
                        
                        current_ptr = func_data.dfg_mut().new_value().get_ptr(current_ptr, index);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(current_ptr).unwrap();
                    } else {
                        // 后续层：对数组类型使用 getelemptr
                        current_ptr = func_data.dfg_mut().new_value().get_elem_ptr(current_ptr, index);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(current_ptr).unwrap();
                    }
                }
                
                // 最后加载目标值
                let load_inst = func_data.dfg_mut().new_value().load(current_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(load_inst).unwrap();
                load_inst
            }
            // endregion 数组访问
            None => panic!("Identifier '{}' not found", lval.ident),
        }
    }
    
    pub fn generate_lval_store(&mut self, lval: &LVal, value: Value) {
        let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();
        
        match symbol_info {
            // 常量不可变
            Some(SymbolInfo::Const(_)) |
            Some(SymbolInfo::LocalConstArray(_, _)) | Some(SymbolInfo::GlobalConstArray(_, _)) => {
                panic!("Cannot assign to constant '{}'", lval.ident);
            }

            // 普通变量赋值
            Some(SymbolInfo::Var(ptr)) | Some(SymbolInfo::GlobalVar(ptr)) => {
                if !lval.indices.is_empty() {
                    panic!("Cannot index into scalar variable");
                }
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let store_inst = func_data.dfg_mut().new_value().store(value, ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();
            }
            
            // 变量数组赋值
            Some(SymbolInfo::LocalArray(ptr, _dimensions)) | Some(SymbolInfo::GlobalArray(ptr, _dimensions)) => {
                if lval.indices.is_empty() {
                    panic!("Cannot assign to entire array '{}'", lval.ident);
                }
                // 数组元素赋值：计算元素地址并存储值
                let elem_ptr = self.generate_array_access_ptr(ptr, &lval.indices);
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let store_inst = func_data.dfg_mut().new_value().store(value, elem_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();
            }

            // TODO: 函数数组参数赋值
            Some(SymbolInfo::ParamArray(param_ptr, _dimensions)) => {
                if lval.indices.is_empty() {
                    panic!("Cannot assign to entire parameter array '{}'", lval.ident);
                }
                // 和取值道理一样，先获取load指针，然后再解引用数组指针(getptr),最后使用getelemptr得到元素指针，再将要赋值的Value赋值给元素
                let param_type = self.function_data_mut().dfg().value(param_ptr).ty().clone();
                println!("Debug: giving val param_type is {:?}", param_type);

                // 先计算所有索引
                let indexes: Vec<Value> = lval.indices
                    .iter()
                    .map(|exp| self.generate_exp(&exp))
                    .collect();

                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();

                // 解引用局部变量，得到真正的指针**i32 -> *i32
                let loaded_ptr = func_data.dfg_mut().new_value().load(param_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(loaded_ptr).unwrap();

                // 从参数指针开始逐层解开
                let mut current_ptr = loaded_ptr;

                for (i, &index) in indexes.iter().enumerate() {
                    let param_type = func_data.dfg().value(current_ptr).ty().clone();
                    println!("Debug: i = {}, current_ptr_type = {}", i, param_type);
                    if i == 0 {
                        // 第一层：对指针类型使用 getptr
                        // loaded_ptr 类型是 *[i32, 3], 使用 getptr 进行指针算术

                        current_ptr = func_data.dfg_mut().new_value().get_ptr(current_ptr, index);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(current_ptr).unwrap();
                    } else {
                        // 后续层：对数组类型使用 getelemptr
                        current_ptr = func_data.dfg_mut().new_value().get_elem_ptr(current_ptr, index);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(current_ptr).unwrap();
                    }
                }

                // 最后赋值给目标值
                let store_inst = func_data.dfg_mut().new_value().store(value, current_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();
            }
            None => {
                panic!("Variable '{}' not found", lval.ident);
            }
        }
    }

    fn generate_array_access_ptr(&mut self, base_ptr: Value, indices: &[Exp]) -> Value {
        let mut ptr = base_ptr;

        // 逐级处理每个索引
        for index_expr in indices {
            // 计算索引值
            let index = self.generate_exp(index_expr);

            // 使用 getelemptr 指令获取元素指针
            let current_bb = self.current_bb();
            let func_data = self.function_data_mut();
            let gep_inst = func_data.dfg_mut().new_value().get_elem_ptr(ptr, index);
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(gep_inst).unwrap();

            ptr = gep_inst;
        }

        ptr
    }


}
