use koopa::ir::{Function, FunctionData, Program, Type};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use crate::lab5plus::ast::CompUnit;
use crate::lab5plus::irgen::symbol::ScopeStack;

pub mod symbol;
pub mod declare;
pub mod block;
pub mod calc;
pub mod statement;
pub mod vars;

pub struct IRGen {
    program:  Program,
    function: Function, // 存储functionId而不是functionData 
    scope_stack: ScopeStack,
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
        }
    }
    
    /// 消耗自身返回koopa::ir::Program
    pub fn generate_koopa_ir(mut self, ast: CompUnit) -> Result<Program, String> {
        let has_return = self.generate_block(&ast.func_def.block);

        // 如果函数没有显式的return语句，添加默认的return 0
        if !has_return {
            let main_data = self.program.func_mut(self.function);
            let zero = main_data.dfg_mut().new_value().integer(0);
            let ret_inst = main_data.dfg_mut().new_value().ret(Some(zero));
            let entry = main_data.layout().entry_bb().unwrap();
            main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret_inst).unwrap();
        }
        
        Ok(self.program)
    }
}