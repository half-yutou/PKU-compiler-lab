use crate::ast::Decl;
use crate::lab8::irgen::symbol::SymbolInfo;
use crate::lab8::irgen::IRGen;
use koopa::ir::builder::LocalInstBuilder;
use koopa::ir::Type;

// 处理声明(常量与变量)
impl IRGen {
    pub fn generate_decl(&mut self, decl: &Decl) {
        match decl {
            // const int a = 1, b = 2 + 3, c = (a > b);
            Decl::Const(const_decl) => {
                for def in &const_decl.const_def_list {
                    // 编译时计算常量值
                    let value = self.evaluate_lor_exp(&def.const_init_val.const_exp.lor_exp);
                    
                    // 检查是否重复定义并存入符号表
                    if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
                        panic!("{}", err)
                    }
                }
            }
            
            // int a, b = 2 + 3, c, d = (a > b) || (c != 0);
            Decl::Var(var_decl) => {
                for def in &var_decl.var_def_list {
                    let unique_name = self.function_irgen.scope_stack.generate_unique_name(&def.ident);

                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();

                    // 为变量分配内存(简单起见，全部分配到栈上-无寄存器分配策略)
                    let alloc_inst = func_data.dfg_mut().new_value().alloc(Type::get_i32());// 返回这个变量的指针
                    func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(alloc_inst).unwrap();
                    
                    // 如果有初始化值，生成store指令
                    if let Some(init_val) = &def.init_val {
                        let init_value = self.generate_exp(&init_val.exp);
                        
                        let current_bb = self.current_bb();// 重新获取current_bb, generate_exp()可能更改了current_bb
                        let func_data = self.function_data_mut();
                        
                        let store_inst = func_data.dfg_mut().new_value().store(init_value, alloc_inst);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap()
                    }
                    
                    // 检查重复定义并存入符号表
                    if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Var(alloc_inst)) {
                        panic!("{}", err)
                    }
                }
            }
        }
    }
}

