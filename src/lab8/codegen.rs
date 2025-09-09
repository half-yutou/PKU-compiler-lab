use std::collections::HashMap;
use koopa::ir::{BinaryOp, FunctionData, Program, Value, ValueKind, BasicBlock};
use koopa::ir::dfg::DataFlowGraph;
use koopa::ir::entities::ValueData;

pub fn generate_riscv_assembly(program: Program) -> String {
    let mut asm = String::new();
    
    // 1. 生成数据段（全局变量）
    let data_section = generate_data_section(&program);
    if !data_section.is_empty() {
        asm.push_str(&data_section);
        asm.push_str("\n");
    }
    
    // 2. 生成代码段
    asm.push_str(".text\n");
    
    for &func_handle in program.func_layout() {
        let func_data = program.func(func_handle);
        
        // 跳过函数声明（库函数声明）
        if func_data.layout().entry_bb().is_none() {
            continue;
        }
        
        let func_name = func_data.name().strip_prefix('@').unwrap_or(func_data.name());

        asm.push_str(&format!(".global {}\n", func_name));
        asm.push_str(&format!("{}:\n", func_name));

        // 生成函数体汇编
        let mut generator = AsmGenerator::new(&program);
        asm.push_str(&generator.gen_function(func_data));
    }
    
    asm
}

// 生成数据段
fn generate_data_section(program: &Program) -> String {
    let mut data_asm = String::new();
    let mut has_globals = false;
    
    // 遍历所有全局值
    for &value_handle in program.inst_layout() {
        let value_data = program.borrow_value(value_handle);
        if let ValueKind::GlobalAlloc(global_alloc) = value_data.kind() {
            if !has_globals {
                data_asm.push_str(".data\n");
                has_globals = true;
            }
            
            // 获取全局变量名
            let var_name = value_data.name()
                .as_ref()
                .unwrap()
                .strip_prefix('@')
                .unwrap();
            
            // 声明全局符号
            data_asm.push_str(&format!(".global {}\n", var_name));
            data_asm.push_str(&format!("{}:\n", var_name));
            
            // 生成初始化数据
            let init_value = global_alloc.init();
            let init_data = program.borrow_value(init_value);
            
            match init_data.kind() {
                ValueKind::Integer(int_val) => {
                    // 使用具体的整数值初始化
                    data_asm.push_str(&format!("  .word {}\n", int_val.value()));
                }
                ValueKind::ZeroInit(_) => {
                    // 零初始化 分配4个字节
                    data_asm.push_str("  .zero 4\n");
                }
                _ => {
                    // 默认零初始化 分配4个字节
                    data_asm.push_str("  .zero 4\n");
                }
            }
        }
    }
    
    data_asm
}

struct AsmGenerator<'a> {
    program: &'a Program,                   // 添加对Program的引用
    stack_size: i32,                        // 当前栈帧大小
    value_stack_map: HashMap<Value, i32>,   // 中间值 -> 栈偏移映射
    bb_param_stack_map: HashMap<(BasicBlock, usize), i32>, // 基本块参数栈映射
    is_leaf_function: bool,                 // 是否为叶子函数
}

impl<'a> AsmGenerator<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            stack_size: 0, 
            value_stack_map: HashMap::new(),
            bb_param_stack_map: HashMap::new(),
            is_leaf_function: true,
        }
    }
    
    pub fn gen_function(&mut self, func_data: &FunctionData) -> String {
        let mut asm = String::new();
        
        // 1. 检测是否为叶子函数
        self.detect_leaf_function(func_data);
        
        // 2. 计算栈帧大小
        self.calculate_stack_size(func_data);

        // 3. 生成函数序言(压栈)
        if self.stack_size > 0 {
            asm.push_str(&format!("  addi  sp, sp, -{}\n", self.stack_size));
            
            // 如果不是叶子函数，保存ra寄存器
            if !self.is_leaf_function {
                asm.push_str(&format!("  sw    ra, {}(sp)\n", self.stack_size - 4));
            }
        }

        // 4. 生成基本块和指令
        let mut is_first_bb = true;
        for (&bb_handle, bb_node) in func_data.layout().bbs() {
            // 第一个基本块不需要额外标签，因为函数名已经是标签
            if !is_first_bb {
                let bb_name = self.get_bb_label(bb_handle);
                asm.push_str(&format!("{}:\n", bb_name));
            }
            is_first_bb = false;
            
            // 处理基本块参数
            let bb_data = func_data.dfg().bb(bb_handle);
            for (i, &param) in bb_data.params().iter().enumerate() {
                if let Some(&stack_offset) = self.bb_param_stack_map.get(&(bb_handle, i)) {
                    self.value_stack_map.insert(param, stack_offset);
                }
            }
            
            // 生成基本块内的指令
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);
                asm.push_str(&self.gen_instruction(inst_handle, value_data, func_data.dfg()));
            }
        }
        
        asm
    }
    
    // 检测是否为叶子函数（不调用其他函数）
    fn detect_leaf_function(&mut self, func_data: &FunctionData) {
        for (&_, bb_node) in func_data.layout().bbs() {
            for &inst_handle in bb_node.insts().keys() {
                let value_data = func_data.dfg().value(inst_handle);
                if matches!(value_data.kind(), ValueKind::Call(_)) {
                    self.is_leaf_function = false;
                    return;
                }
            }
        }
    }

    fn calculate_stack_size(&mut self, func_data: &FunctionData) {
        self.stack_size = 0;
        self.value_stack_map.clear();
        self.bb_param_stack_map.clear();

        // 首先为函数参数分配栈空间
        for (_, &param) in func_data.params().iter().enumerate() {
            let offset = self.stack_size;
            self.value_stack_map.insert(param, offset);
            self.stack_size += 4;
        }

        // 处理所有基本块的参数（phi 节点参数）
        for (&bb_handle, _) in func_data.layout().bbs() {
            let bb_data = func_data.dfg().bb(bb_handle);
            for (i, &param) in bb_data.params().iter().enumerate() {
                let offset = self.stack_size;
                self.bb_param_stack_map.insert((bb_handle, i), offset);
                self.value_stack_map.insert(param, offset);
                self.stack_size += 4;
            }
        }

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
                    ValueKind::Call(_) => {
                        // 函数调用结果分配（如果有返回值）
                        if !matches!(value_data.ty().kind(), koopa::ir::TypeKind::Unit) {
                            let offset = self.stack_size;
                            self.value_stack_map.insert(inst_handle, offset);
                            self.stack_size += 4;
                        }
                    },
                    ValueKind::Integer(_) | ValueKind::FuncArgRef(_) => {
                        // 常量和函数参数不需要额外栈空间
                    },
                    ValueKind::Return(_) | ValueKind::Store(_) | ValueKind::Branch(_) | ValueKind::Jump(_) => {
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

        // 如果不是叶子函数，需要额外空间保存ra寄存器
        if !self.is_leaf_function {
            self.stack_size += 4;
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
            ValueKind::FuncArgRef(_) => {
                // 函数参数引用不需要生成指令
                // 参数值在函数入口处已经通过寄存器或栈传递
                String::new()
            }
            ValueKind::BlockArgRef(_) => {
                // 基本块参数引用不需要生成指令
                // 参数值已经通过跳转时的寄存器传递或栈传递
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
            ValueKind::Call(call) => {
                let mut asm = String::new();
                
                // 获取被调用函数的句柄和参数
                let callee = call.callee();
                let args = call.args();
                
                // 通过program获取被调用函数的名称
                let callee_data = self.program.func(callee);
                let func_name = callee_data.name().strip_prefix('@').unwrap_or(callee_data.name());
                
                // 准备参数
                // 前8个参数通过a0-a7寄存器传递
                for (i, &arg) in args.iter().enumerate().take(8) {
                    asm.push_str(&self.load_value_to_reg(arg, &format!("a{}", i), dfg));
                }
                
                // 第9个及以后的参数通过栈传递（从右到左压栈）
                if args.len() > 8 {
                    let stack_args = &args[8..];
                    let stack_space = ((stack_args.len() * 4 + 15) / 16) * 16; // 16字节对齐
                    
                    // 调整栈指针为栈参数分配空间
                    asm.push_str(&format!("  addi  sp, sp, -{}\n", stack_space));
                    
                    // 从右到左压栈（最后一个参数先压栈）
                    for (i, &arg) in stack_args.iter().enumerate().rev() {
                        asm.push_str(&self.load_value_to_reg(arg, "t0", dfg));
                        let offset = i * 4;
                        asm.push_str(&format!("  sw    t0, {}(sp)\n", offset));
                    }
                }
                
                // 调用函数
                asm.push_str(&format!("  call  {}\n", func_name));
                
                // 恢复栈指针（如果有栈参数）
                if args.len() > 8 {
                    let stack_args = &args[8..];
                    let stack_space = ((stack_args.len() * 4 + 15) / 16) * 16;
                    asm.push_str(&format!("  addi  sp, sp, {}\n", stack_space));
                }
                
                // 如果函数有返回值，将a0的值保存到栈
                if !matches!(value_data.ty().kind(), koopa::ir::TypeKind::Unit) {
                    if let Some(&offset) = self.value_stack_map.get(&inst_handle) {
                        asm.push_str(&format!("  sw    a0, {}(sp)\n", offset));
                    }
                }
                
                asm
            }
            ValueKind::Return(ret) => {
                let mut asm = String::new();

                // 如果有返回值，将其加载到a0寄存器
                if let Some(return_value) = ret.value() {
                    asm.push_str(&self.load_value_to_reg(return_value, "a0", dfg));
                }

                // 恢复ra寄存器（如果不是叶子函数）
                if !self.is_leaf_function && self.stack_size > 0 {
                    asm.push_str(&format!("  lw    ra, {}(sp)\n", self.stack_size - 4));
                }

                // 恢复栈指针
                if self.stack_size > 0 {
                    asm.push_str(&format!("  addi  sp, sp, {}\n", self.stack_size));
                }

                // 返回
                asm.push_str("  ret\n");

                asm
            }
            ValueKind::Branch(branch) => {
                let mut asm = String::new();
                
                // 加载条件值到寄存器
                asm.push_str(&self.load_value_to_reg(branch.cond(), "t0", dfg));
                
                // 生成条件分支指令
                let true_label = self.get_bb_label(branch.true_bb());
                let false_label = self.get_bb_label(branch.false_bb());
                
                asm.push_str(&format!("  bnez  t0, {}\n", true_label));
                asm.push_str(&format!("  j     {}\n", false_label));
                
                asm
            }
            ValueKind::Jump(jump) => {
                let mut asm = String::new();
                
                // 处理跳转参数传递
                for (i, &arg) in jump.args().iter().enumerate() {
                    if let Some(&stack_offset) = self.bb_param_stack_map.get(&(jump.target(), i)) {
                        asm.push_str(&self.load_value_to_reg(arg, "t0", dfg));
                        asm.push_str(&format!("  sw    t0, {}(sp)\n", stack_offset));
                    }
                }
                
                let target_label = self.get_bb_label(jump.target());
                asm.push_str(&format!("  j     {}\n", target_label));
                asm
            }
            ValueKind::Alloc(_) => {
                // alloc 指令不生成实际汇编代码，只记录栈偏移映射
                // 映射关系已在 calculate_stack_size 中建立
                String::new()
            }
            ValueKind::Store(store) => {
                let mut asm = String::new();

                // 先加载要存储的值到t0
                asm.push_str(&self.load_value_to_reg(store.value(), "t0", dfg));

                // 检查目标是否为全局变量
                if dfg.values().contains_key(&store.dest()) {
                    // 局部变量（在函数 dfg 中）
                    let dest_value_data = dfg.value(store.dest());
                    match dest_value_data.kind() {
                        ValueKind::Alloc(_) => {
                            // 目标是Alloc分配的栈地址，直接存储到栈偏移位置
                            if let Some(&offset) = self.value_stack_map.get(&store.dest()) {
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
                } else {
                    // 全局变量（不在函数 dfg 中）
                    let dest_value_ref = self.program.borrow_value(store.dest());
                    match dest_value_ref.kind() {
                        ValueKind::GlobalAlloc(_) => {
                            // 目标是全局变量，使用la指令加载地址，然后sw存储值
                            let var_name = dest_value_ref.name()
                                .as_ref()
                                .unwrap()
                                .strip_prefix('@')
                                .unwrap();
                            asm.push_str(&format!("  la    t1, {}\n", var_name));
                            asm.push_str("  sw    t0, 0(t1)\n");
                        },
                        _ => {
                            // 其他类型的地址，先加载地址到t1，再存储
                            asm.push_str(&self.load_value_to_reg(store.dest(), "t1", dfg));
                            asm.push_str("  sw    t0, 0(t1)\n");
                        }
                    }
                }

                asm
            }
            ValueKind::Load(load) => {
                let mut asm = String::new();

                // 检查源是否为全局变量
                if dfg.values().contains_key(&load.src()) {
                    // 局部变量（在函数 dfg 中）
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
                } else {
                    // 全局变量（不在函数 dfg 中）
                    let src_value_ref = self.program.borrow_value(load.src());
                    match src_value_ref.kind() {
                        ValueKind::GlobalAlloc(_) => {
                            // 源是全局变量，使用la指令加载地址，然后lw加载值
                            let var_name = src_value_ref.name()
                                .as_ref()
                                .unwrap()
                                .strip_prefix('@')
                                .unwrap();
                            asm.push_str(&format!("  la    t1, {}\n", var_name));
                            asm.push_str("  lw    t0, 0(t1)\n");
                        },
                        _ => {
                            // 其他类型的地址，先加载地址到t1，再从该地址加载值
                            asm.push_str(&self.load_value_to_reg(load.src(), "t1", dfg));
                            asm.push_str("  lw    t0, 0(t1)\n");
                        }
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
            ValueKind::FuncArgRef(arg_ref) => {
                let arg_index = arg_ref.index();
                if arg_index < 8 {
                    // 前8个参数通过a0-a7寄存器传递
                    format!("  mv    {}, a{}\n", target_reg, arg_index)
                } else {
                    // 第9个及以后的参数从栈中获取
                    // 参数在调用者栈帧中，需要计算正确的偏移
                    let offset = self.stack_size + (arg_index as i32 - 8) * 4;
                    format!("  lw    {}, {}(sp)\n", target_reg, offset)
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

    // 生成基本块标签的辅助方法
    fn get_bb_label(&self, bb: BasicBlock) -> String {
        // 将 BasicBlock 转换为字符串，然后清理特殊字符
        let bb_str = format!("{:?}", bb);
        // 移除 % 前缀和括号，只保留数字部分(riscv不支持label带小括号，故重新命名)
        let cleaned = bb_str
            .strip_prefix('%')
            .unwrap_or(&bb_str)
            .replace("BasicBlock", "")
            .replace("(", "")
            .replace(")", "");
        format!("LBB{}", cleaned)
    }
}