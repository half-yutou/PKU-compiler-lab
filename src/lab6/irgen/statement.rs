use crate::ast::Stmt;

use crate::lab6::irgen::symbol::SymbolInfo;
use crate::lab6::irgen::vars::generate_exp;
use crate::lab6::irgen::{IRGen, ControlFlowType};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder};

impl IRGen {
    pub fn generate_stmt(&mut self, stmt: &Stmt) -> bool{
        match stmt {
            Stmt::If(cond, then_stmt, else_stmt) => {
                // 生成条件表达式的值
                let cond_value = {
                    let func_data = self.program.func_mut(self.function);
                    generate_exp(cond, func_data, &mut self.scope_stack, self.current_bb.unwrap())
                };
                
                self.bb_counter += 1;
                // 创建基本块
                let (then_bb, else_bb, end_bb) = {
                    let func_data = self.program.func_mut(self.function);
                    let then_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%then_{}", self.bb_counter)));
                    let else_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%else_{}", self.bb_counter)));
                    let end_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%end_{}", self.bb_counter)));
                    
                    // 将基本块添加到函数布局中
                    func_data.layout_mut().bbs_mut().push_key_back(then_bb).unwrap();
                    func_data.layout_mut().bbs_mut().push_key_back(else_bb).unwrap();
                    func_data.layout_mut().bbs_mut().push_key_back(end_bb).unwrap();
                    
                    (then_bb, else_bb, end_bb)
                };
                
                // 推入控制流上下文
                self.push_control_flow(end_bb, ControlFlowType::IfElse);
                
                // 在当前基本块生成条件分支指令
                {
                    let func_data = self.program.func_mut(self.function);
                    let current_bb = self.current_bb.unwrap_or_else(|| func_data.layout().entry_bb().unwrap());
                    let br_inst = func_data.dfg_mut().new_value().branch(cond_value, then_bb, else_bb);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(br_inst).unwrap();
                }
                
                // 处理then分支
                self.current_bb = Some(then_bb);
                if let Stmt::Block(block) = then_stmt.as_ref() {
                    self.generate_block(block);
                } else {
                    // 如果不是块语句，需要创建一个临时作用域
                    self.scope_stack.enter_scope();
                    self.generate_stmt(then_stmt);
                    self.scope_stack.exit_scope();
                }
                // 检查then_bb是否已有终结指令，如果没有则添加跳转
                {
                    let func_data = self.program.func_mut(self.function);
                    let bb_data = func_data.layout().bbs().node(&then_bb).unwrap();
                    let has_terminator = bb_data.insts().back_key()
                        .map(|inst| {
                            let value_data = func_data.dfg().value(*inst);
                            matches!(value_data.kind(), 
                                koopa::ir::ValueKind::Return(_) | 
                                koopa::ir::ValueKind::Jump(_) | 
                                koopa::ir::ValueKind::Branch(_))
                        })
                        .unwrap_or(false);
                    
                    if !has_terminator {
                        let jump_inst = func_data.dfg_mut().new_value().jump(end_bb);
                        func_data.layout_mut().bb_mut(then_bb).insts_mut().push_key_back(jump_inst).unwrap();
                    }
                }
                

                
                // 处理else分支
                self.current_bb = Some(else_bb);
                match else_stmt {
                    Some(stmt) => {
                        match stmt.as_ref() {
                            Stmt::Block(block) => {
                                self.generate_block(block);
                            }
                            _ => {
                                self.scope_stack.enter_scope();
                                self.generate_stmt(stmt);
                                self.scope_stack.exit_scope();
                            }
                        }
                        // 检查else_bb是否已有终结指令(如ret,提前返回了)，如果没有则添加跳转(jump)
                        {
                            let func_data = self.program.func_mut(self.function);
                            let bb_data = func_data.layout().bbs().node(&else_bb).unwrap();
                            let has_terminator = bb_data.insts().back_key()
                                .map(|inst| {
                                    let value_data = func_data.dfg().value(*inst);
                                    matches!(value_data.kind(), 
                                    koopa::ir::ValueKind::Return(_) | 
                                    koopa::ir::ValueKind::Jump(_) | 
                                    koopa::ir::ValueKind::Branch(_))
                                })
                                .unwrap_or(false);

                            if !has_terminator {
                                let jump_inst = func_data.dfg_mut().new_value().jump(end_bb);
                                func_data.layout_mut().bb_mut(else_bb).insts_mut().push_key_back(jump_inst).unwrap();
                            }
                        }
                    }
                    None => {
                        // 空的else分支，直接跳转到end_bb
                        let func_data = self.program.func_mut(self.function);
                        let jump_inst = func_data.dfg_mut().new_value().jump(end_bb);
                        func_data.layout_mut().bb_mut(else_bb).insts_mut().push_key_back(jump_inst).unwrap();
                    }
                }
                
                // 设置当前基本块为end_bb
                self.current_bb = Some(end_bb);
                
                // 记录延迟跳转：如果当前end_bb为空且有外层控制流，记录跳转映射
                self.record_pending_jump(end_bb);
                
                // 弹出控制流上下文
                self.pop_control_flow();
                
                false
            }
            
            Stmt::Assign(lval, exp) => {
                // 根据右侧表达式求值
                let value = {
                    let func_data = self.program.func_mut(self.function);
                    generate_exp(exp, func_data, &mut self.scope_stack, self.current_bb.unwrap())
                };
                
                // 获取左值的指针
                match self.scope_stack.lookup(&lval.ident) {
                    Some(SymbolInfo::Var(ptr)) => {
                        let func_data = self.program.func_mut(self.function);
                        // 生成 store 指令
                        let store_inst = func_data.dfg_mut().new_value().store(value, *ptr);
                        let current_bb = self.current_bb.unwrap_or_else(|| func_data.layout().entry_bb().unwrap());
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();
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
                let func_data = self.program.func_mut(self.function);
                let current_bb = self.current_bb.unwrap_or_else(|| func_data.layout().entry_bb().unwrap());
                
                match exp_opt {
                    Some(exp) => {
                        // `return 1`有返回值的return语句
                        let value = generate_exp(exp, func_data, &mut self.scope_stack, current_bb);
                        let ret_inst = func_data.dfg_mut().new_value().ret(Some(value));
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(ret_inst).unwrap();
                    }
                    None => {
                        // `return` 无返回值的return语句
                        let ret_inst = func_data.dfg_mut().new_value().ret(None);
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(ret_inst).unwrap();
                    }
                }
                true
            }
            
            Stmt::Exp(exp_opt) => {
                match exp_opt {
                    Some(exp) => {
                        // `1+2;`表达式语句，生成IR但是丢弃结果
                        let func_data = self.program.func_mut(self.function);
                        generate_exp(exp, func_data, &mut self.scope_stack, self.current_bb.unwrap());
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
        }
    }
}