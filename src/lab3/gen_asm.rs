use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::{Program, Value, ValueKind, BinaryOp, FunctionData};
use std::collections::HashMap;
use koopa::ir::entities::ValueData;

pub fn generate_riscv_assembly(program: Program) -> String {
    let mut asm = String::new();
    asm.push_str(".text\n");

    // 遍历所有函数
    for &func_handle in program.func_layout() {
        let func_data = program.func(func_handle);
        let func_name = func_data.name().strip_prefix('@').unwrap_or(func_data.name());

        asm.push_str(&format!(".global {}\n", func_name));
        asm.push_str(&format!("{}:\n", func_name));

        // 生成函数体汇编
        let mut generator = AssemblyGenerator::new();
        asm.push_str(&generator.gen_function(func_data));
    }

    asm
}

struct AssemblyGenerator {
    value_map: HashMap<Value, String>, // Value -> 寄存器映射
    temp_counter: usize,               // 临时寄存器计数器
}

impl AssemblyGenerator {
    fn new() -> Self {
        Self {
            value_map: HashMap::new(),
            temp_counter: 0,
        }
    }

    fn gen_function(&mut self, func_data: &FunctionData) -> String {
        let mut asm = String::new();
        
        // 生成实际指令
        for (&_, bb_node) in func_data.layout().bbs() {
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);
                asm.push_str(&self.gen_instruction(inst_handle, value_data, func_data.dfg()));
            }
        }
        
        asm
    }
    

    fn gen_instruction(&mut self, value: Value, value_data: &ValueData, dfg: &DataFlowGraph) -> String {
        match value_data.kind() {
            ValueKind::Integer(i) => {
                // 如果是非零立即数且还没有生成过 li 指令
                if i.value() != 0 {
                    let reg = self.get_value_reg(value, dfg);
                    format!("  li    {}, {}\n", reg, i.value())
                } else {
                    String::new() // 零值不需要生成指令
                }
            },
            
            ValueKind::Binary(binary) => {
                // 确保操作数的 li 指令已生成
                let mut asm = String::new();
                
                // 为左操作数生成 li 指令（如果需要）
                let lhs_data = dfg.value(binary.lhs());
                if let ValueKind::Integer(i) = lhs_data.kind() {
                    if i.value() != 0 && !self.value_map.contains_key(&binary.lhs()) {
                        asm.push_str(&self.gen_instruction(binary.lhs(), lhs_data, dfg));
                    }
                }
                
                // 为右操作数生成 li 指令（如果需要）
                let rhs_data = dfg.value(binary.rhs());
                if let ValueKind::Integer(i) = rhs_data.kind() {
                    if i.value() != 0 && !self.value_map.contains_key(&binary.rhs()) {
                        asm.push_str(&self.gen_instruction(binary.rhs(), rhs_data, dfg));
                    }
                }
                
                // 生成二元运算指令
                let lhs_reg = self.get_value_reg(binary.lhs(), dfg);
                let rhs_reg = self.get_value_reg(binary.rhs(), dfg);
                let result_reg = self.alloc_temp_reg();
                self.value_map.insert(value, result_reg.clone());
                
                match binary.op() {
                    BinaryOp::Eq => {
                        asm.push_str(&format!("  xor   {}, {}, {}\n  seqz  {}, {}\n", 
                                       result_reg, lhs_reg, rhs_reg, result_reg, result_reg));
                    },
                    BinaryOp::Sub => {
                        asm.push_str(&format!("  sub   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    _ => {}, // 其他运算符暂不实现
                }
                
                asm
            },
            
            ValueKind::Return(ret) => {
                if let Some(ret_val) = ret.value() {
                    let ret_reg = self.get_value_reg(ret_val, dfg);
                    format!("  mv    a0, {}\n  ret\n", ret_reg)
                } else {
                    "  ret\n".to_string()
                }
            },

            _ => String::new(), // 其他指令类型暂不处理
        }
    }

    // 分配临时寄存器t0-t6
    fn alloc_temp_reg(&mut self) -> String {
        let reg = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        reg
    }

    fn get_value_reg(&mut self, value: Value, dfg: &DataFlowGraph) -> String {
        if let Some(reg) = self.value_map.get(&value) {
            reg.clone()
        } else {
            let value_data = dfg.value(value);
            match value_data.kind() {
                ValueKind::Integer(i) => {
                    if i.value() == 0 {
                        "x0".to_string()  // 零值直接使用零寄存器
                    } else {
                        // 非零立即数：分配寄存器并记录映射
                        let reg = self.alloc_temp_reg();
                        self.value_map.insert(value, reg.clone());
                        reg
                    }
                },
                _ => {
                    panic!("Unhandled value type in get_value_reg");
                }
            }
        }
    }
}