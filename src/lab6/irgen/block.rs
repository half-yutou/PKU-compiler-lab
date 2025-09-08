use crate::ast::{Block, BlockItem};
use crate::lab6::irgen::IRGen;

impl IRGen {
    pub fn generate_block(&mut self, block: &Block) -> bool {
        // 进入新的作用域{}
        self.scope_stack.enter_scope();
        
        let mut has_return = false;
        for block_item in &block.block_item_list {
            if has_return {
                break;
            }
            match block_item {
                BlockItem::Decl(decl) => self.generate_decl(decl), 
                BlockItem::Stmt(stmt) => {
                    has_return = self.generate_stmt(stmt);
                }
            }
        }
        
        // 退出当前作用域{}
        self.scope_stack.exit_scope();
        has_return
    }


}