use crate::lab5plus::ast::Stmt;
use crate::lab5plus::irgen::vars::generate_exp;
use crate::lab5plus::irgen::IRGen;
use koopa::ir::builder::LocalInstBuilder;
use crate::lab5plus::irgen::symbol::SymbolInfo;

impl IRGen {
    pub fn generate_stmt(&mut self, stmt: &Stmt) -> bool{
        let func_data = self.program.func_mut(self.function);
        match stmt {
            Stmt::Assign(lval, exp) => {
                // 根据右侧表达式求值
                let value = generate_exp(exp, func_data, &mut self.scope_stack);

                // 获取左值的指针
                match self.scope_stack.lookup(&lval.ident) {
                    Some(SymbolInfo::Var(ptr)) => {
                        // 生成 store 指令
                        let store_inst = func_data.dfg_mut().new_value().store(value, *ptr);
                        let entry  =func_data.layout().entry_bb().unwrap();
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
                    }
                    Some(SymbolInfo::Const(_)) => {
                        panic!("Cannot assign to constant '{}'!", lval.ident);
                    }
                    None => {
                        panic!("Variable '{}' not found!", lval.ident);
                    }
                }
                false
            }
            Stmt::Return(exp_opt) => {
                match exp_opt {
                    Some(exp) => {
                        // `return 1`有返回值的return语句
                        let value = generate_exp(exp, func_data, &mut self.scope_stack);
                        let ret_inst = func_data.dfg_mut().new_value().ret(Some(value));
                        let entry = func_data.layout_mut().entry_bb().unwrap();
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret_inst).unwrap();
                    }
                    None => {
                        // `return` 无返回值的return语句
                        let ret_inst = func_data.dfg_mut().new_value().ret(None);
                        let entry  =func_data.layout().entry_bb().unwrap();
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret_inst).unwrap();
                    }
                }
                true // 遇到了return语句，告知上层
            }
            Stmt::Exp(exp_opt) => {
                match exp_opt {
                    Some(exp) => {
                        // `1+2;`表达式语句，生成IR但是丢弃结果
                        generate_exp(exp, func_data, &mut self.scope_stack);
                    }
                    None => {
                        // `;`空语句
                    }
                }
                false
            }
            Stmt::Block(block) => {
                // 嵌套代码块，递归处理
                self.generate_block(block)
            }
            _ => false
        }
    }
}