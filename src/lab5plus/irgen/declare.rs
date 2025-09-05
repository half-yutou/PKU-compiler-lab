use koopa::ir::builder::LocalInstBuilder;
use koopa::ir::{FunctionData, Type, Value};
use crate::ast::{ConstExp, Decl, Exp};
use crate::lab5plus::irgen::symbol::{ScopeStack, SymbolInfo};
use crate::lab5plus::irgen::{calc, vars, IRGen};

// 处理声明(常量与变量)
impl IRGen {
    pub fn generate_decl(&mut self, decl: &Decl) {
        match decl {
            // const int a = 1, b = 2 + 3, c = (a > b);
            Decl::Const(const_decl) => {
                for def in &const_decl.const_def_list {
                    // 编译时计算常量值
                    let value = evaluate_const_exp(&def.const_init_val.const_exp, &self.scope_stack);
                    
                    // 检查是否重复定义并存入符号表
                    if let Err(err) = self.scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
                        panic!("{}", err)
                    }
                }
            }
            
            // int a, b = 2 + 3, c;
            Decl::Var(var_decl) => {
                for def in &var_decl.var_def_list {
                    let unique_name = self.scope_stack.generate_unique_name(&def.ident);
                    
                    let func_data = self.program.func_mut(self.function);
                    let entry = func_data.layout().entry_bb().unwrap();

                    // 为变量分配内存(简单起见，全部分配到栈上)
                    let alloc_inst = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                    func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
                    
                    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc_inst).unwrap();
                    
                    // 如果有初始化值，生成store指令
                    if let Some(init_val) = &def.init_val {
                        let init_value = generate_var_exp(&init_val.exp, func_data, &mut self.scope_stack);
                        let store_inst = func_data.dfg_mut().new_value().store(init_value, alloc_inst);
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap()
                    }
                    
                    // 检查重复定义并存入符号表
                    if let Err(err) = self.scope_stack.define(def.ident.clone(), SymbolInfo::Var(alloc_inst)) {
                        panic!("{}", err)
                    }
                    
                }
            }
        }
    }
}

pub fn evaluate_const_exp(const_exp: &ConstExp, scope_stack: &ScopeStack) -> i32{
    calc::evaluate_lor_exp(&const_exp.lor_exp, scope_stack)
}

pub fn generate_var_exp(var_exp: &Exp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    vars::generate_exp(var_exp, func_data, scope_stack)
}

