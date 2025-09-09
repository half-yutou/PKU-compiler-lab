use crate::ast::Stmt;

use crate::lab9::irgen::symbol::SymbolInfo;
use crate::lab9::irgen::{ControlFlowType, IRGen, LoopContext};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder};

impl IRGen {
    pub fn generate_stmt(&mut self, stmt: &Stmt) -> bool{
        match stmt {
            Stmt::Break => {
                if let Some(loop_context) = self.function_irgen.loop_stack.last() {
                    let loop_end = loop_context.loop_end;

                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();

                    let jump_inst = func_data.dfg_mut().new_value().jump(loop_end);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(jump_inst).unwrap();
                    true // 表示已添加终结指令
                } else {
                    panic!("break statement outside of loop");
                }
            }
            
            Stmt::Continue => {
                if let Some(loop_context) = self.function_irgen.loop_stack.last() {
                    let loop_header = loop_context.loop_header;

                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    
                    let jump_inst = func_data.dfg_mut().new_value().jump(loop_header);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(jump_inst).unwrap();
                    true // 表示已添加终结指令
                } else {
                    panic!("continue statement outside of loop");
                }
            }
            
            Stmt::While(cond, stmt) => {
                self.function_irgen.bb_counter += 1;
                let bb_counter = self.function_irgen.bb_counter;

                // 创建循环相关的基本块
                let (loop_header, loop_body, loop_end) = {
                    let func_data = self.function_data_mut();
                    
                    let loop_header = func_data.dfg_mut().new_bb().basic_block(Some(format!("%loop_header_{}", bb_counter)));
                    let loop_body = func_data.dfg_mut().new_bb().basic_block(Some(format!("%loop_body_{}", bb_counter)));
                    let loop_end = func_data.dfg_mut().new_bb().basic_block(Some(format!("%loop_end_{}", bb_counter)));
                    
                    // 将基本块添加到函数布局中
                    func_data.layout_mut().bbs_mut().push_key_back(loop_header).unwrap();
                    func_data.layout_mut().bbs_mut().push_key_back(loop_body).unwrap();
                    func_data.layout_mut().bbs_mut().push_key_back(loop_end).unwrap();
                    
                    (loop_header, loop_body, loop_end)
                };
                
                // 从当前基本块跳转到循环头
                if let Some(current_bb) = self.function_irgen.current_bb {
                    let func_data = self.function_data_mut();
                    let jump_inst = func_data.dfg_mut().new_value().jump(loop_header);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(jump_inst).unwrap();
                }

                // 推入循环上下文和控制流上下文
                self.function_irgen.loop_stack.push(LoopContext {
                    loop_header,
                    loop_end,
                });
                self.push_control_flow(loop_end, ControlFlowType::While {
                    loop_header,
                    loop_end,
                });
                
                // 设置当前基本块=循环头，生成条件判断
                self.function_irgen.current_bb = Some(loop_header);
                let cond_value = self.generate_exp(cond);
                
                // 生成条件分支
                {
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let branch_inst = func_data.dfg_mut().new_value().branch(cond_value, loop_body, loop_end);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(branch_inst).unwrap();
                }
                
                // 设置当前基本块=循环体
                self.function_irgen.current_bb = Some(loop_body);
                
                // 生成循环体语句
                let has_terminator = if let Stmt::Block(block) = stmt.as_ref() {
                    self.generate_block(block)
                } else {
                    self.function_irgen.scope_stack.enter_scope();
                    let result = self.generate_stmt(stmt);
                    self.function_irgen.scope_stack.exit_scope();
                    result
                };                
                // 如果循环体没有终结指令，添加跳转到循环头
                if !has_terminator {
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let jump_inst = func_data.dfg_mut().new_value().jump(loop_header);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(jump_inst).unwrap();
                }
                
                // 设置当前基本块为循环结束
                self.function_irgen.current_bb = Some(loop_end);
                
                // 记录延迟跳转：如果当前loop_end有外层控制流，记录跳转映射
                self.record_pending_jump(loop_end);
                
                // 弹出上下文
                self.function_irgen.loop_stack.pop();
                self.pop_control_flow();
                
                false // while语句本身不是终结指令
            }
            
            Stmt::If(cond, then_stmt, else_stmt) => {
                // 生成条件表达式的值
                let cond_value = self.generate_exp(cond);
                
                self.function_irgen.bb_counter += 1;
                let bb_counter = self.function_irgen.bb_counter;
                // 创建基本块
                let (then_bb, else_bb, end_bb) = {
                    let func_data = self.function_data_mut();
                    
                    let then_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%then_{}", bb_counter)));
                    let else_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%else_{}", bb_counter)));
                    let end_bb = func_data.dfg_mut().new_bb().basic_block(Some(format!("%end_{}", bb_counter)));
                    
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
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let br_inst = func_data.dfg_mut().new_value().branch(cond_value, then_bb, else_bb);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(br_inst).unwrap();
                }
                
                // 处理then分支
                self.function_irgen.current_bb = Some(then_bb);
                if let Stmt::Block(block) = then_stmt.as_ref() {
                    self.generate_block(block);
                } else {
                    // 如果不是块语句，需要创建一个临时作用域
                    self.function_irgen.scope_stack.enter_scope();
                    self.generate_stmt(then_stmt);
                    self.function_irgen.scope_stack.exit_scope();
                }
                // 检查then_bb是否已有终结指令，如果没有则添加跳转
                {
                    let func_data = self.function_data_mut();
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
                self.function_irgen.current_bb = Some(else_bb);
                match else_stmt {
                    Some(stmt) => {
                        if let Stmt::Block(block) = stmt.as_ref() {
                            self.generate_block(block);
                        } else {
                            // 如果不是块语句，需要创建一个临时作用域
                            self.function_irgen.scope_stack.enter_scope();
                            self.generate_stmt(stmt);
                            self.function_irgen.scope_stack.exit_scope();
                        }
                        
                        // 检查else_bb是否已有终结指令，如果没有则添加跳转
                        {
                            let func_data = self.function_data_mut();
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
                        let func_data = self.function_data_mut();
                        let jump_inst = func_data.dfg_mut().new_value().jump(end_bb);
                        func_data.layout_mut().bb_mut(else_bb).insts_mut().push_key_back(jump_inst).unwrap();
                    }
                }
                
                // 设置当前基本块为end_bb
                self.function_irgen.current_bb = Some(end_bb);
                
                // 记录延迟跳转：如果当前end_bb为空且有外层控制流，记录跳转映射
                self.record_pending_jump(end_bb);
                
                // 弹出控制流上下文
                self.pop_control_flow();
                
                false
            }
            
            Stmt::Assign(lval, exp) => {
                // 根据右侧表达式求值
                let value = self.generate_exp(exp);
                
                // 获取左值的指针
                let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();
                match symbol_info {
                    Some(SymbolInfo::Var(ptr)) | Some(SymbolInfo::GlobalVar(ptr))=> {
                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();
                        // 生成 store 指令
                        let store_inst = func_data.dfg_mut().new_value().store(value, ptr);
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
                match exp_opt {
                    Some(exp) => {
                        // `return 1`有返回值的return语句
                        let value = self.generate_exp(exp);
                        
                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();
                        
                        let ret_inst = func_data.dfg_mut().new_value().ret(Some(value));
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(ret_inst).unwrap();
                    }
                    None => {
                        // `return` 无返回值的return语句
                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();
                        
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
                        self.generate_exp(exp);
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