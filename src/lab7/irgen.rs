use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use crate::lab7::ast::CompUnit;
use crate::lab7::irgen::symbol::ScopeStack;
use std::collections::HashMap;

/// 控制流上下文，用于跟踪嵌套的if-else结构
#[derive(Debug, Clone)]
pub struct ControlFlowContext {
    pub end_bb: BasicBlock,  // 当前控制流的结束基本块
    pub context_type: ControlFlowType,
}

/// 循环上下文，用于break/continue跳转
#[derive(Debug, Clone)]
pub struct LoopContext {
    pub loop_header: BasicBlock,  // continue跳转目标
    pub loop_end: BasicBlock,     // break跳转目标
}

#[derive(Debug, Clone)]
pub enum ControlFlowType {
    IfElse,  // if-else语句
    While {  // while循环
        loop_header: BasicBlock,  // 循环头（条件检查）
        loop_end: BasicBlock,     // 循环结束
    },
}

pub mod symbol;
pub mod declare;
pub mod block;
pub mod calc;
pub mod statement;
pub mod vars;

pub struct IRGen {
    program: Program,
    function: Function,
    scope_stack: ScopeStack,
    current_bb: BasicBlock,
    bb_counter: u32,
    control_flow_stack: Vec<ControlFlowContext>,
    pending_jumps: HashMap<BasicBlock, BasicBlock>,
    loop_stack: Vec<LoopContext>, // 新增：循环上下文栈
}

impl IRGen {
    /// 构建一个"有main函数与entry块"Program的IRGen
    pub fn new() -> Self {
        let mut program = Program::new();
        let main_func = program.new_func(FunctionData::with_param_names(
            "@main".into(),
            vec![],
            Type::get_i32(),
        ));
        let main_data_mut = program.func_mut(main_func);

        let entry = main_data_mut.dfg_mut().new_bb().basic_block(Some("%entry".into()));
        main_data_mut.layout_mut().bbs_mut().extend([entry]);

        Self {
            program,
            function: main_func, 
            scope_stack: ScopeStack::new(),
            current_bb: entry,
            bb_counter: 0,
            control_flow_stack: Vec::new(),
            pending_jumps: HashMap::new(),
            loop_stack: Vec::new(), 
        }
    }
    
    pub fn generate_koopa_ir(mut self, ast: CompUnit) -> Result<Program, String> {
        self.generate_block(&ast.func_def.block);
        // 处理所有延迟跳转
        self.process_pending_jumps();
        self.ensure_terminator(self.current_bb);
        Ok(self.program)
    }
    
    /// 推入控制流上下文到栈中
    pub fn push_control_flow(&mut self, end_bb: BasicBlock, context_type: ControlFlowType) {
        self.control_flow_stack.push(ControlFlowContext {
            end_bb,
            context_type,
        });
    }
    
    /// 从栈中弹出控制流上下文
    pub fn pop_control_flow(&mut self) -> Option<ControlFlowContext> {
        self.control_flow_stack.pop()
    }
    
    /// 记录延迟跳转：如果当前end_bb有外层控制流，就记录跳转映射
    pub fn record_pending_jump(&mut self, current_end_bb: BasicBlock) {
        if self.control_flow_stack.len() > 1 {
            // 取倒数第二个作为外层end_bb（最后一个是当前的）
            if let Some(outer_context) = self.control_flow_stack.get(self.control_flow_stack.len() - 2) {
                self.pending_jumps.insert(current_end_bb, outer_context.end_bb);
            }
        }
    }
    
    /// 在函数结束时处理所有延迟跳转
    pub fn process_pending_jumps(&mut self) {
        let pending_jumps = std::mem::take(&mut self.pending_jumps);
        
        for (from_bb, to_bb) in pending_jumps {
            let func_data = self.program.func_mut(self.function);
            let bb_data = func_data.layout().bbs().node(&from_bb).unwrap();
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
                let jump_inst = func_data.dfg_mut().new_value().jump(to_bb);
                func_data.layout_mut().bb_mut(from_bb).insts_mut().push_key_back(jump_inst).unwrap();
            }
        }
    }
    /// 确保指定基本块有终结指令，如果没有则添加默认的ret 0
    pub fn ensure_terminator(&mut self, bb: BasicBlock) {
        let main_data = self.program.func_mut(self.function);
        let bb_data = main_data.layout().bbs().node(&bb).unwrap();
        let has_terminator = bb_data.insts().back_key()
            .map(|inst| {
                let value_data = main_data.dfg().value(*inst);
                matches!(value_data.kind(), 
                    koopa::ir::ValueKind::Return(_) | 
                    koopa::ir::ValueKind::Jump(_) | 
                    koopa::ir::ValueKind::Branch(_))
            })
            .unwrap_or(false);

        if !has_terminator {
            let zero = main_data.dfg_mut().new_value().integer(0);
            let ret_inst = main_data.dfg_mut().new_value().ret(Some(zero));
            main_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(ret_inst).unwrap();
        }
    }

}