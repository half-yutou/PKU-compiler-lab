use koopa::ir::BasicBlock;
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::ast::{Block, BlockItem};
use crate::lab6::irgen::IRGen;

impl IRGen {
    pub fn generate_block(&mut self, block: &Block) {
        // 进入新的作用域{}
        self.scope_stack.enter_scope();
        
        for block_item in &block.block_item_list {
            
            match block_item {
                BlockItem::Decl(decl) => self.generate_decl(decl), 
                BlockItem::Stmt(stmt) => {
                    self.generate_stmt(stmt);
                }
            }
        }
        
        // 退出当前作用域{}
        self.scope_stack.exit_scope();
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