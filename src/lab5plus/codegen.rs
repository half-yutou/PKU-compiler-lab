use std::collections::HashMap;
use koopa::ir::{BinaryOp, FunctionData, Program, Value, ValueKind};
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;

pub fn generate_riscv_assembly(program: Program) -> String {
    let mut asm = String::new();
    asm.push_str(".text\n");
    
    for &func_handle in program.func_layout() {
        let func_data = program.func(func_handle);
        let func_name = func_data.name().strip_prefix('@').unwrap_or(func_data.name());

        asm.push_str(&format!(".global {}\n", func_name));
        asm.push_str(&format!("{}:\n", func_name));

        // 生成函数体汇编
        let mut generator = AsmGenerator::new();
        asm.push_str(&generator.gen_function(func_data));
    }
    
    asm
}

struct AsmGenerator {
    stack_size: i32,                        // 当前栈帧大小
    value_stack_map: HashMap<Value, i32>,   // 中间值 -> 栈偏移映射
}

impl AsmGenerator {
    pub fn new() -> Self {
        Self {
            stack_size: 0, 
            value_stack_map: HashMap::new(), 
        }
    }
    
    pub fn gen_function(&mut self, func_data: &FunctionData) -> String {
        let mut asm = String::new();
        
        // 1. 计算栈帧大小
        self.calculate_stack_size(func_data);

        // 2. 生成函数序言(压栈)
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

    fn calculate_stack_size(&mut self, func_data: &FunctionData) {
        self.stack_size = 0;
        self.value_stack_map.clear();

        // 遍历所有指令，为每个产生值的指令分配栈空间
        for (&_, bb_node) in func_data.layout().bbs() {
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);

                match value_data.kind() {
                    ValueKind::Alloc(_) => {
                        // 变量分配
                        let offset = self.stack_size;
                        self.value_stack_map.insert(inst_handle, offset);
                        self.stack_size += 4;
                    },
                    ValueKind::Binary(_) => {
                        // 二元运算结果分配
                        let offset = self.stack_size;
                        self.value_stack_map.insert(inst_handle, offset);
                        self.stack_size += 4;
                    },
                    ValueKind::Load(_) => {
                        // 加载结果分配
                        let offset = self.stack_size;
                        self.value_stack_map.insert(inst_handle, offset);
                        self.stack_size += 4;
                    },
                    ValueKind::Integer(_) => {
                        // 常量不需要栈空间，可以直接用 li 指令或 x0 寄存器
                    },
                    ValueKind::Return(_) | ValueKind::Store(_) => {
                        // 这些指令不产生需要存储的值
                    },
                    _ => {
                        // 其他可能产生值的指令也分配栈空间
                        let offset = self.stack_size;
                        self.value_stack_map.insert(inst_handle, offset);
                        self.stack_size += 4;
                    }
                }
            }
        }

        // 16字节对齐
        if self.stack_size > 0 {
            self.stack_size = (self.stack_size + 15) & !15;
        }
    }
    
    fn gen_instruction(&mut self, inst_handle: Value, value_data: &ValueData, dfg: &DataFlowGraph) -> String {

        match value_data.kind() { 
            ValueKind::Integer(_) => { 
                // integer 指令说明是常量数字
                // 常量数字的加载不需要具体指令，在load_value_to_reg调用时会生成将数字加载到寄存器的指令
                String::new()
            }
            ValueKind::Binary(binary) => {
                let mut asm = String::new();

                // 1. 加载左操作数到 t0
                asm.push_str(&self.load_value_to_reg(binary.lhs(), "t0", dfg));

                // 2. 加载右操作数到 t1
                asm.push_str(&self.load_value_to_reg(binary.rhs(), "t1", dfg));

                // 3. 执行运算，结果存到 t2
                match binary.op() {
                    BinaryOp::Add => asm.push_str("  add   t2, t0, t1\n"),
                    BinaryOp::Sub => asm.push_str("  sub   t2, t0, t1\n"),
                    BinaryOp::Mul => asm.push_str("  mul   t2, t0, t1\n"),
                    BinaryOp::Div => asm.push_str("  div   t2, t0, t1\n"),
                    BinaryOp::Mod => asm.push_str("  rem   t2, t0, t1\n"),

                    // 比较运算
                    BinaryOp::Eq => {
                        asm.push_str("  xor   t2, t0, t1\n");
                        asm.push_str("  seqz  t2, t2\n");
                    },
                    BinaryOp::NotEq => {
                        asm.push_str("  xor   t2, t0, t1\n");
                        asm.push_str("  snez  t2, t2\n");
                    },
                    BinaryOp::Lt => asm.push_str("  slt   t2, t0, t1\n"),
                    BinaryOp::Le => {
                        asm.push_str("  slt   t2, t1, t0\n");
                        asm.push_str("  seqz  t2, t2\n");
                    },
                    BinaryOp::Gt => asm.push_str("  slt   t2, t1, t0\n"),
                    BinaryOp::Ge => {
                        asm.push_str("  slt   t2, t0, t1\n");
                        asm.push_str("  seqz  t2, t2\n");
                    },

                    // 位运算（用于逻辑运算）
                    BinaryOp::And => asm.push_str("  and   t2, t0, t1\n"),
                    BinaryOp::Or  => asm.push_str("  or    t2, t0, t1\n"),

                    _ => panic!("Unsupported binary operation: {:?}", binary.op()),
                }

                // 4. 将结果存储到栈
                if let Some(&offset) = self.value_stack_map.get(&inst_handle) {
                    asm.push_str(&format!("  sw    t2, {}(sp)\n", offset));
                } else {
                    panic!("Binary result not found in stack map: {:?}", inst_handle);
                }

                asm
            }
            ValueKind::Return(ret) => {
                let mut asm = String::new();

                // 如果有返回值，将其加载到a0寄存器
                if let Some(return_value) = ret.value() {
                    asm.push_str(&self.load_value_to_reg(return_value, "a0", dfg));
                }

                // 恢复栈指针
                if self.stack_size > 0 {
                    asm.push_str(&format!("  addi  sp, sp, {}\n", self.stack_size));
                }

                // 返回
                asm.push_str("  ret\n");

                asm
            }
            ValueKind::Alloc(_) => {
                // alloc 指令不生成实际汇编代码，只记录栈偏移映射
                // 映射关系已在 calculate_stack_size 中建立
                String::new()
            }
            ValueKind::Store(store) => {
                let mut asm = String::new();

                // 将要存储的值加载到临时寄存器t0
                asm.push_str(&self.load_value_to_reg(store.value(), "t0", dfg));

                // 获取目标地址到临时寄存器t1
                let dest_value_data = dfg.value(store.dest());
                match dest_value_data.kind() {
                    ValueKind::Alloc(_) => {
                        // 目标是Alloc分配的栈地址
                        if let Some(&offset) = self.value_stack_map.get(&store.dest()) {
                            // 直接存储到栈偏移位置
                            asm.push_str(&format!("  sw    t0, {}(sp)\n", offset));
                        } else {
                            panic!("Alloc destination not found in stack map: {:?}", store.dest());
                        }
                    },
                    _ => {
                        // 其他类型的地址，先加载地址到t1，再存储
                        asm.push_str(&self.load_value_to_reg(store.dest(), "t1", dfg));
                        asm.push_str("  sw    t0, 0(t1)\n");
                    }
                }

                asm
            }
            ValueKind::Load(load) => {
                let mut asm = String::new();

                // 获取源地址
                let src_value_data = dfg.value(load.src());
                match src_value_data.kind() {
                    ValueKind::Alloc(_) => {
                        // 源是Alloc分配的栈地址，直接从栈加载
                        if let Some(&src_offset) = self.value_stack_map.get(&load.src()) {
                            asm.push_str(&format!("  lw    t0, {}(sp)\n", src_offset));
                        } else {
                            panic!("Alloc source not found in stack map: {:?}", load.src());
                        }
                    },
                    _ => {
                        // 其他类型的地址，先加载地址到t1，再从该地址加载值
                        asm.push_str(&self.load_value_to_reg(load.src(), "t1", dfg));
                        asm.push_str("  lw    t0, 0(t1)\n");
                    }
                }

                // 将加载的值存储到栈上（Load指令的结果）
                if let Some(&offset) = self.value_stack_map.get(&inst_handle) {
                    asm.push_str(&format!("  sw    t0, {}(sp)\n", offset));
                } else {
                    panic!("Load result not found in stack map: {:?}", inst_handle);
                }

                asm
            }
             _ => String::new(),
        }
        
    }

    // 将值加载到指定寄存器的辅助方法
    fn load_value_to_reg(&self, value: Value, target_reg: &str, dfg: &DataFlowGraph) -> String {
        let value_data = dfg.value(value);
        match value_data.kind() {
            ValueKind::Integer(i) => {
                if i.value() == 0 { // x0寄存器永远为0
                    format!("  mv    {}, x0\n", target_reg)
                } else {
                    format!("  li    {}, {}\n", target_reg, i.value())
                }
            },
            _ => {
                // 从栈加载其他类型的值
                if let Some(&offset) = self.value_stack_map.get(&value) {
                    format!("  lw    {}, {}(sp)\n", target_reg, offset)
                } else {
                    panic!("Value not found in stack map: {:?}", value);
                }
            }
        }
    }
}