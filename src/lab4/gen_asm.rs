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
    value_map: HashMap<Value, String>,     // Value -> 寄存器映射
    alloc_map: HashMap<Value, i32>,       // alloc -> 栈偏移映射
    stack_size: i32,                      // 当前栈帧大小
    temp_counter: usize,                  // 临时寄存器计数器
    free_regs: Vec<String>,              // 可重用的寄存器池
}

impl AssemblyGenerator {
    fn new() -> Self {
        Self {
            value_map: HashMap::new(),
            alloc_map: HashMap::new(),
            stack_size: 0,
            temp_counter: 0,
            free_regs: Vec::new(),
        }
    }

    fn gen_function(&mut self, func_data: &FunctionData) -> String {
        let mut asm = String::new();
        
        // 1. 计算栈帧大小
        self.calculate_stack_size(func_data);
        
        // 2. 生成函数序言
        if self.stack_size > 0 {
            asm.push_str(&format!("  addi  sp, sp, -{}\n", self.stack_size));
        }
        
        // 3. 生成实际指令
        for (&_, bb_node) in func_data.layout().bbs() {
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);
                asm.push_str(&self.gen_instruction(inst_handle, value_data, func_data.dfg()));
            }
        }
        
        asm
    }
    
    // 计算栈帧大小并建立 alloc 映射
    fn calculate_stack_size(&mut self, func_data: &FunctionData) {
        self.stack_size = 0;
        self.alloc_map.clear();
        
        // 遍历所有基本块和指令，查找 alloc 指令
        for (&_, bb_node) in func_data.layout().bbs() {
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);
                if let ValueKind::Alloc(_) = value_data.kind() {
                    let offset = self.stack_size;
                    self.alloc_map.insert(inst_handle, offset);
                    self.stack_size += 4; // 每个 i32 变量占 4 字节
                }
            }
        }
        
        // 确保栈对齐（16字节对齐）
        if self.stack_size > 0 {
            self.stack_size = (self.stack_size + 15) & !15;
        }
    }

    fn gen_instruction(&mut self, value: Value, value_data: &ValueData, dfg: &DataFlowGraph) -> String {
        match value_data.kind() {
            ValueKind::Integer(i) => {
                // 总是为非零立即数生成li指令，确保寄存器中有正确的值
                if i.value() != 0 {
                    let reg = self.get_value_reg(value, dfg);
                    format!("  li    {}, {}\n", reg, i.value())
                } else {
                    // 零值也需要确保在需要时能正确处理
                    String::new()
                }
            },
            
            ValueKind::Alloc(_) => {
                // alloc 指令不生成实际汇编代码，只记录栈偏移映射
                // 映射关系已在 calculate_stack_size 中建立
                String::new()
            },
            
            ValueKind::Load(load) => {
                let ptr = load.src();
                if let Some(&offset) = self.alloc_map.get(&ptr) {
                    let result_reg = self.alloc_temp_reg();
                    self.value_map.insert(value, result_reg.clone());
                    format!("  lw    {}, {}(sp)\n", result_reg, offset)
                } else {
                    panic!("Load from unknown alloc: {:?}", ptr);
                }
            },
            
            ValueKind::Store(store) => {
                let mut asm = String::new();
                
                // 确保源值的指令已生成（特别是 li 指令）
                let src_value = store.value();
                let src_data = dfg.value(src_value);
                if let ValueKind::Integer(i) = src_data.kind() {
                    if i.value() != 0 && !self.value_map.contains_key(&src_value) {
                        asm.push_str(&self.gen_instruction(src_value, src_data, dfg));
                    }
                }
                
                let src_reg = self.get_value_reg(src_value, dfg);
                let ptr = store.dest();
                if let Some(&offset) = self.alloc_map.get(&ptr) {
                    asm.push_str(&format!("  sw    {}, {}(sp)\n", src_reg, offset));
                    asm
                } else {
                    panic!("Store to unknown alloc: {:?}", ptr);
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
                    // 算术运算
                    BinaryOp::Add => {
                        asm.push_str(&format!("  add   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Sub => {
                        asm.push_str(&format!("  sub   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Mul => {
                        asm.push_str(&format!("  mul   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Div => {
                        asm.push_str(&format!("  div   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Mod => {
                        asm.push_str(&format!("  rem   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    
                    // 比较运算 - 使用真实的RISC-V指令
                    BinaryOp::Eq => {
                        asm.push_str(&format!("  xor   {}, {}, {}\n  seqz  {}, {}\n", 
                                       result_reg, lhs_reg, rhs_reg, result_reg, result_reg));
                    },
                    BinaryOp::NotEq => {
                        asm.push_str(&format!("  xor   {}, {}, {}\n  snez  {}, {}\n", 
                                       result_reg, lhs_reg, rhs_reg, result_reg, result_reg));
                    },
                    BinaryOp::Lt => {
                        // a < b：直接使用slt指令
                        asm.push_str(&format!("  slt   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Le => {
                        // a <= b 等价于 !(a > b) 等价于 !(b < a)
                        asm.push_str(&format!("  slt   {}, {}, {}\n  seqz  {}, {}\n", 
                                       result_reg, rhs_reg, lhs_reg, result_reg, result_reg));
                    },
                    BinaryOp::Gt => {
                        // a > b 等价于 b < a
                        asm.push_str(&format!("  slt   {}, {}, {}\n", result_reg, rhs_reg, lhs_reg));
                    },
                    BinaryOp::Ge => {
                        // a >= b 等价于 !(a < b)
                        asm.push_str(&format!("  slt   {}, {}, {}\n  seqz  {}, {}\n", 
                                       result_reg, lhs_reg, rhs_reg, result_reg, result_reg));
                    },
                    
                    // 位运算（用于逻辑运算）
                    BinaryOp::And => {
                        asm.push_str(&format!("  and   {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOp::Or => {
                        asm.push_str(&format!("  or    {}, {}, {}\n", result_reg, lhs_reg, rhs_reg));
                    },
                    
                    _ => {
                        panic!("Unsupported binary operation: {:?}", binary.op());
                    }
                }
                
                // 释放操作数寄存器（如果它们是临时计算结果）
                self.maybe_free_operand(binary.lhs(), dfg);
                self.maybe_free_operand(binary.rhs(), dfg);
                
                asm
            },
            
            ValueKind::Return(ret) => {
                let mut asm = String::new();
                
                if let Some(ret_val) = ret.value() {
                    let value_data = dfg.value(ret_val);
                    
                    // 确保返回值已经正确处理（生成必要的指令）
                    if let ValueKind::Integer(i) = value_data.kind() {
                        if i.value() != 0 && !self.value_map.contains_key(&ret_val) {
                            asm.push_str(&self.gen_instruction(ret_val, value_data, dfg));
                        }
                    }
                    
                    let ret_reg = self.get_value_reg(ret_val, dfg);
                    asm.push_str(&format!("  mv    a0, {}\n", ret_reg));
                }
                
                // 生成函数尾声（恢复栈指针）
                if self.stack_size > 0 {
                    asm.push_str(&format!("  addi  sp, sp, {}\n", self.stack_size));
                }
                
                asm.push_str("  ret\n");
                asm
            }

            _ => String::new(), // 其他指令类型暂不处理
        }
    }

    // 分配临时寄存器，支持寄存器重用
    fn alloc_temp_reg(&mut self) -> String {
        // 优先使用释放的寄存器
        if let Some(reg) = self.free_regs.pop() {
            return reg;
        }
        
        // 可用寄存器：t0-t6, a1-a7 (a0用于返回值)
        let available_regs = [
            "t0", "t1", "t2", "t3", "t4", "t5", "t6",
            "a1", "a2", "a3", "a4", "a5", "a6", "a7"
        ];
        
        if self.temp_counter >= available_regs.len() {
            // 如果寄存器不够，强制重用最早的寄存器
            let reg_idx = self.temp_counter % available_regs.len();
            available_regs[reg_idx].to_string()
        } else {
            let reg = available_regs[self.temp_counter].to_string();
            self.temp_counter += 1;
            reg
        }
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
                    panic!("Unhandled value type in get_value_reg: {:?}", value_data.kind());
                }
            }
        }
    }
    
    // 尝试释放操作数寄存器
    fn maybe_free_operand(&mut self, value: Value, dfg: &DataFlowGraph) {
        let value_data = dfg.value(value);
        match value_data.kind() {
            ValueKind::Integer(_) => {
                // 立即数不需要释放寄存器，因为可以重新生成 li 指令
            },
            ValueKind::Binary(_) | ValueKind::Load(_) => {
                // 二元运算和 load 的结果可以释放
                if let Some(reg) = self.value_map.remove(&value) {
                    if reg != "x0" && reg != "a0" {
                        self.free_regs.push(reg);
                    }
                }
            },
            _ => {}
        }
    }
}