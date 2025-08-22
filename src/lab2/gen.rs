use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;
use koopa::ir::values::Return;
use koopa::ir::ValueKind;
use koopa::ir::{FunctionData, Program};

pub fn generate_riskv_assembly(program: Program) -> String {
    let mut asm = String::new();
    asm.push_str(".text\n");
    
    // 遍历所有函数(为了顺序遍历，必须遍历layout而非funcs)
    for &func_handle in program.func_layout() {
        let func_data = program.func(func_handle);
        asm.push_str(&func_data.gen_assembly());
    }
    
    asm
}

/// 不需要 DFG 的汇编生成 trait
trait GenAssembly {
    fn gen_assembly(&self) -> String;
}

/// 需要 DFG 的汇编生成 trait
trait GenAssemblyWithDFG {
    fn gen_assembly_with_dfg(&self, dfg: &DataFlowGraph) -> String;
}

/// 为 FunctionData 实现汇编生成
impl GenAssembly for FunctionData {
    fn gen_assembly(&self) -> String {
        let mut asm = String::new();
        let func_name = &self.name()[1..]; // 去掉 @ 前缀
        
        // 生成函数标签
        asm.push_str(&format!(".global {}\n", func_name));
        asm.push_str(&format!("{}:\n", func_name));
        
        // 遍历所有基本块
        for (&bb_handle, bb_node) in self.layout().bbs() {
            // 从布局中获取基本块的指令列表
            for &inst_handle in bb_node.insts().keys() {
                let inst_data = self.dfg().value(inst_handle);
                asm.push_str(&inst_data.gen_assembly_with_dfg(self.dfg()));
            }
        }
        
        asm
    }
}

/// 为 ValueData 实现汇编生成（需要 DFG 上下文）
impl GenAssemblyWithDFG for ValueData {
    fn gen_assembly_with_dfg(&self, dfg: &DataFlowGraph) -> String {
        match self.kind() {
            ValueKind::Return(ret_data) => {
                generate_return_instruction(ret_data, dfg)
            },
            ValueKind::Integer(_) => {
                // 整数常量不直接生成汇编，作为操作数使用
                String::new()
            },
            // 可以继续添加其他指令类型
            _ => {
                // 暂时不处理的指令类型
                String::new()
            }
        }
    }
}

/// 生成 return 指令的汇编代码
fn generate_return_instruction(ret_data: &Return, dfg: &DataFlowGraph) -> String {
    let mut asm = String::new();
    
    // 处理返回值
    if let Some(ret_value_handle) = ret_data.value() {
        let ret_value_data = dfg.value(ret_value_handle);
        match ret_value_data.kind() {
            ValueKind::Integer(int_data) => {
                // 将立即数加载到 a0 寄存器（RISC-V 返回值寄存器）
                asm.push_str(&format!("li a0, {}\n", int_data.value()));
            },
            _ => {
                // 处理其他类型的返回值（变量、表达式结果等）
                asm.push_str("  # TODO: handle other return value types\n");
            }
        }
    }
    
    // 生成返回指令
    asm.push_str("ret\n");
    asm
}
