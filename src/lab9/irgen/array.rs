use crate::ast::{ConstInitVal, InitVal};
use crate::lab9::irgen::IRGen;
use koopa::ir::{Type, TypeKind, Value};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};

/// 局部数组初始化器枚举，用于处理局部数组初始化
#[derive(Debug, Clone)]
pub enum LocalInitializer {
    Const(i32),
    Value(Value),
    List(Vec<LocalInitializer>),
}

impl LocalInitializer {
    /// 从AST的ConstInitVal创建LocalInitializer（用于局部常量数组）
    pub fn from_const_init_val(init_val: &ConstInitVal, irgen: &mut IRGen) -> Result<Self, String> {
        match init_val {
            ConstInitVal::Exp(const_exp) => {
                let value = irgen.evaluate_lor_exp(&const_exp.lor_exp);
                Ok(Self::Const(value))
            }
            ConstInitVal::List(list) => {
                let inits: Result<Vec<_>, _> = list
                    .iter()
                    .map(|v| Self::from_const_init_val(v, irgen))
                    .collect();
                Ok(Self::List(inits?))
            }
        }
    }

    /// 从AST的InitVal创建LocalInitializer（用于局部变量数组）
    pub fn from_init_val(init_val: &InitVal, irgen: &mut IRGen) -> Result<Self, String> {
        match init_val {
            InitVal::Exp(exp) => {
                let value = irgen.generate_exp(exp);
                Ok(Self::Value(value))
            }
            InitVal::List(list) => {
                let inits: Result<Vec<_>, _> = list
                    .iter()
                    .map(|v| Self::from_init_val(v, irgen))
                    .collect();
                Ok(Self::List(inits?))
            }
        }
    }
    
    /// 根据给定类型重塑初始化器
    pub fn reshape(self, ty: &Type) -> Result<Self, String> {
        // 获取维度列表
        let mut lens = Vec::new();
        
        let mut current_ty = ty;
        loop {
            match current_ty.kind() {
                TypeKind::Int32 => break,
                TypeKind::Array(base, len) => {
                    lens.push(*len);
                    current_ty = base;
                }
                _ => return Err("Unsupported type for array initialization".to_string()),
            }
        }
        
        // 计算累积长度
        let mut last_len = 1;
        let lens: Vec<_> = lens
            .into_iter()
            .rev()
            .map(|l| {
                last_len *= l;
                (l, last_len)
            })
            .collect();
        
        // 执行重塑
        match self {
            Self::Const(val) if lens.is_empty() => Ok(Self::Const(val)),
            Self::Value(val) if lens.is_empty() => Ok(Self::Value(val)),
            Self::List(l) if !lens.is_empty() => Self::reshape_impl(l, &lens),
            _ => Err("Invalid initialization".to_string()),
        }
    }
    
    fn reshape_impl(inits: Vec<Self>, lens: &[(usize, usize)]) -> Result<Self, String> {
        let mut reshaped: Vec<Vec<Self>> = (0..=lens.len()).map(|_| Vec::new()).collect();
        let mut len = 0;
        
        // 处理初始化器元素
        for init in inits {
            if len >= lens.last().unwrap().1 {
                return Err("Too many initializer elements".to_string());
            }
            match init {
                Self::List(list) => {
                    let next_lens = match reshaped.iter().position(|v| !v.is_empty()) {
                        Some(0) => return Err("Misaligned initialization".to_string()),
                        Some(i) => &lens[..i],
                        None => &lens[..lens.len() - 1],
                    };
                    reshaped[next_lens.len()].push(Self::reshape_impl(list, next_lens)?);
                    Self::carry(&mut reshaped, lens);
                    len += next_lens.last().unwrap().1;
                }
                _ => {
                    reshaped[0].push(init);
                    Self::carry(&mut reshaped, lens);
                    len += 1;
                }
            }
        }
        
        // 填充零
        while len < lens.last().unwrap().1 {
            reshaped[0].push(Self::Const(0));
            Self::carry(&mut reshaped, lens);
            len += 1;
        }
        
        Ok(reshaped.pop().unwrap().pop().unwrap())
    }
    
    fn carry(reshaped: &mut [Vec<Self>], lens: &[(usize, usize)]) {
        for (i, &(len, _)) in lens.iter().enumerate() {
            if reshaped[i].len() == len {
                let init = Self::List(reshaped[i].drain(..).collect());
                reshaped[i + 1].push(init);
            }
        }
    }

    /// 将初始化器扁平化为值列表（用于局部数组初始化）
    pub fn flatten(self, irgen: &mut IRGen) -> Vec<Value> {
        match self {
            Self::Const(val) => {
                let func_data = irgen.function_data_mut();
                let const_val = func_data.dfg_mut().new_value().integer(val);
                vec![const_val]
            }
            Self::Value(val) => vec![val],
            Self::List(list) => {
                list.into_iter()
                    .flat_map(|init| init.flatten(irgen))
                    .collect()
            }
        }
    }
}

impl IRGen {
    /// 创建函数级别的零值
    pub fn create_zero_value(&mut self) -> Value {
        let func_data = self.function_data_mut();
        let zero = func_data.dfg_mut().new_value().integer(0);
        zero
    }

    /// 评估常量初始化列表，展开为一维数组
    pub fn evaluate_const_init_list(&mut self, init_list: &Vec<crate::ast::ConstInitVal>, dimensions: &[usize]) -> Vec<i32> {
        let total_size = dimensions.iter().product::<usize>();
        let mut flat_values = Vec::new();
        
        self.flatten_const_init_list(init_list, &mut flat_values);
        
        // 如果初始化值不足，用0填充到总大小
        while flat_values.len() < total_size {
            flat_values.push(0);
        }
        
        // 如果初始化值过多，截断到总大小
        flat_values.truncate(total_size);
        
        flat_values
    }

    /// 递归展开常量初始化列表为一维数组
    fn flatten_const_init_list(&mut self, init_list: &Vec<crate::ast::ConstInitVal>, result: &mut Vec<i32>) {
        for init_val in init_list {
            match init_val {
                crate::ast::ConstInitVal::Exp(exp) => {
                    let value = self.evaluate_lor_exp(&exp.lor_exp);
                    result.push(value);
                }
                crate::ast::ConstInitVal::List(sub_list) => {
                    // 递归处理嵌套列表
                    self.flatten_const_init_list(sub_list, result);
                }
            }
        }
    }
    
    /// 初始化局部数组，使用 getelemptr 和 store 指令
    pub fn initialize_local_array(&mut self, array_ptr: Value, values: &[Value], dimensions: &[usize]) {
        let mut flat_index = 0;
        let mut indices = vec![0; dimensions.len()];
        
        // 使用多重循环遍历所有数组位置
        self.initialize_array_recursive(array_ptr, values, dimensions, &mut indices, 0, &mut flat_index);
    }

    /// 递归初始化数组的各个元素
    fn initialize_array_recursive(
        &mut self, 
        array_ptr: Value, 
        values: &[Value], 
        dimensions: &[usize], 
        indices: &mut Vec<usize>, 
        depth: usize, 
        flat_index: &mut usize
    ) {
        if depth == dimensions.len() {
            // 到达最深层，存储值
            if *flat_index < values.len() {
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();

                // 计算元素地址：从外层到内层逐步使用 getelemptr
                let mut ptr = array_ptr;
                for (_dim_idx, &index) in indices.iter().enumerate() {
                    let idx_value = func_data.dfg_mut().new_value().integer(index as i32);
                    let elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(ptr, idx_value);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(elem_ptr).unwrap();
                    ptr = elem_ptr;
                }

                // 存储值
                let store_inst = func_data.dfg_mut().new_value().store(values[*flat_index], ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();

                *flat_index += 1;
            }
        } else {
            // 递归处理下一维
            for i in 0..dimensions[depth] {
                indices[depth] = i;
                self.initialize_array_recursive(array_ptr, values, dimensions, indices, depth + 1, flat_index);
            }
        }
    }

    /// 初始化局部常量数组，使用 getelemptr 和 store 指令
    pub fn initialize_local_const_array(&mut self, array_ptr: Value, values: &[i32], dimensions: &[usize]) {
        let mut flat_index = 0;
        let mut indices = vec![0; dimensions.len()];
        
        // 使用多重循环遍历所有数组位置
        self.initialize_const_array_recursive(array_ptr, values, dimensions, &mut indices, 0, &mut flat_index);
    }

    /// 递归初始化常量数组的各个元素
    fn initialize_const_array_recursive(
        &mut self, 
        array_ptr: Value, 
        values: &[i32], 
        dimensions: &[usize], 
        indices: &mut Vec<usize>, 
        depth: usize, 
        flat_index: &mut usize
    ) {
        if depth == dimensions.len() {
            // 到达最深层，存储值
            if *flat_index < values.len() {
                // 先创建常量值
                let const_value = self.program.new_value().integer(values[*flat_index]);
                
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();

                // 计算元素地址：从外层到内层逐步使用 getelemptr
                let mut ptr = array_ptr;
                for (_dim_idx, &index) in indices.iter().enumerate() {
                    let idx_value = func_data.dfg_mut().new_value().integer(index as i32);
                    let elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(ptr, idx_value);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(elem_ptr).unwrap();
                    ptr = elem_ptr;
                }

                // 存储常量值
                let store_inst = func_data.dfg_mut().new_value().store(const_value, ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap();

                *flat_index += 1;
            }
        } else {
            // 递归处理下一维
            for i in 0..dimensions[depth] {
                indices[depth] = i;
                self.initialize_const_array_recursive(array_ptr, values, dimensions, indices, depth + 1, flat_index);
            }
        }
    }

    /// 为局部数组分配内存
    pub fn alloc_local_array(&mut self, array_type: Type) -> Value {
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        let alloc_inst = func_data.dfg_mut().new_value().alloc(array_type);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(alloc_inst).unwrap();
        
        alloc_inst
    }
}