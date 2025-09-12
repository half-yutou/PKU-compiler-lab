use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use koopa::ir::Value;
use crate::ast::{AddExp, EqExp, Exp, LAndExp, LOrExp, LVal, MulExp, PrimaryExp, RelExp, UnaryExp};
use crate::lab9::irgen::IRGen;
use crate::lab9::irgen::symbol::SymbolInfo;

impl IRGen {
    // 在 IRGen impl 块中添加
    pub fn generate_arg_exp(&mut self, exp: &Exp) -> Value {
        // 首先尝试解析是否为 LVal
        if let Some(lval) = self.try_extract_lval(exp) {
            // 查询符号表获取类型信息
            let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();

            match symbol_info {
                // 情况1：数组类型且无索引 -> 数组传参
                Some(SymbolInfo::LocalArray(_, _)) |
                Some(SymbolInfo::GlobalArray(_, _)) |
                Some(SymbolInfo::ParamArray(_, _)) => {
                    return if lval.indices.is_empty() {
                        // 数组传参：返回数组首地址
                        self.generate_lval_as_param(&lval)
                    } else {
                        // 数组访问作为参数：需要判断是否为子数组传参
                        self.generate_lval_as_arg(&lval)
                    }
                }

                // 情况2：普通变量
                Some(SymbolInfo::Var(_)) | Some(SymbolInfo::GlobalVar(_)) => {
                    if !lval.indices.is_empty() {
                        panic!("Cannot index into scalar variable '{}'", lval.ident);
                    }
                    return self.generate_lval_load(&lval);
                }

                // 情况3：常量
                Some(SymbolInfo::Const(_)) => {
                    if !lval.indices.is_empty() {
                        panic!("Cannot index into constant '{}'", lval.ident);
                    }
                    return self.generate_lval_load(&lval);
                }

                _ => panic!("Identifier '{}' not found", lval.ident),
            }
        }

        // 如果不是 LVal，按普通表达式处理
        self.generate_exp(exp)
    }

    // 处理数组访问作为参数的情况
    pub fn generate_lval_as_arg(&mut self, lval: &LVal) -> Value {
        let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();
        
        match symbol_info {
            Some(SymbolInfo::LocalConstArray(ptr, dimensions)) |
            Some(SymbolInfo::GlobalConstArray(ptr, dimensions)) |
            Some(SymbolInfo::LocalArray(ptr, dimensions)) |
            Some(SymbolInfo::GlobalArray(ptr, dimensions)) => {
                // 检查访问后的维度
                let remaining_dims = dimensions.len() - lval.indices.len();
                
                if remaining_dims > 1 {
                    // 剩余维度大于1，需要降维处理
                    let array_ptr = self.generate_array_access_ptr(ptr, &lval.indices);
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let first_elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(array_ptr, zero);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(first_elem_ptr).unwrap();
                    first_elem_ptr
                } else if remaining_dims == 1 {
                    // 剩余维度等于1，返回子数组首元素指针
                    let array_ptr = self.generate_array_access_ptr(ptr, &lval.indices);
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let first_elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(array_ptr, zero);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(first_elem_ptr).unwrap();
                    first_elem_ptr
                } else {
                    // 没有剩余维度，返回标量值
                    self.generate_lval_load(lval)
                }
            }
            
            Some(SymbolInfo::ParamArray(param_ptr, dimensions)) => {
                // 检查访问后的维度
                let remaining_dims = dimensions.len() - lval.indices.len();
                
                if remaining_dims > 1 {
                    // 剩余维度大于1，需要降维处理
                    let array_ptr = self.generate_param_array_access_ptr(param_ptr, &lval.indices);
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let first_elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(array_ptr, zero);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(first_elem_ptr).unwrap();
                    first_elem_ptr
                } else if remaining_dims == 1 {
                    // 剩余维度等于1，返回子数组首元素指针
                    let array_ptr = self.generate_param_array_access_ptr(param_ptr, &lval.indices);
                    let current_bb = self.current_bb();
                    let func_data = self.function_data_mut();
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let first_elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(array_ptr, zero);
                    func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(first_elem_ptr).unwrap();
                    first_elem_ptr
                } else {
                    // 没有剩余维度，返回标量值
                    self.generate_lval_load(lval)
                }
            }
            
            _ => {
                // 非数组类型，直接加载值
                self.generate_lval_load(lval)
            }
        }
    }
    
    // 处理参数数组的访问，返回指针而不是值
    pub fn generate_param_array_access_ptr(&mut self, param_ptr: Value, indices: &[crate::ast::Exp]) -> Value {
        // 先计算所有索引值
        let index_values: Vec<Value> = indices.iter().map(|exp| self.generate_exp(exp)).collect();
        
        let current_bb = self.current_bb();
        let func_data = self.function_data_mut();
        
        // 先加载参数指针
        let loaded_ptr = func_data.dfg_mut().new_value().load(param_ptr);
        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(loaded_ptr).unwrap();
        
        let mut current_ptr = loaded_ptr;
        
        // 处理索引
        for (i, &index) in index_values.iter().enumerate() {
            if i == 0 {
                // 第一层使用 getptr
                current_ptr = func_data.dfg_mut().new_value().get_ptr(current_ptr, index);
            } else {
                // 后续层使用 getelemptr
                current_ptr = func_data.dfg_mut().new_value().get_elem_ptr(current_ptr, index);
            }
            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(current_ptr).unwrap();
        }
        
        current_ptr
    }

    // 处理数组作为参数传递时返回数组首地址
    pub fn generate_lval_as_param(&mut self, lval: &LVal) -> Value {
        let symbol_info = self.function_irgen.scope_stack.lookup(&lval.ident).cloned();
        
        match symbol_info {
            // 局部数组：返回数组首地址
            Some(SymbolInfo::LocalArray(ptr, _)) | Some(SymbolInfo::GlobalArray(ptr, _)) => {
                // 使用 get_elem_ptr 获取数组首地址（索引为0）
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let zero = func_data.dfg_mut().new_value().integer(0);
                let first_elem_ptr = func_data.dfg_mut().new_value().get_elem_ptr(ptr, zero);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(first_elem_ptr).unwrap();
                first_elem_ptr
            }
            
            // 参数数组：返回加载后的指针值
            Some(SymbolInfo::ParamArray(param_ptr, _)) => {
                let current_bb = self.current_bb();
                let func_data = self.function_data_mut();
                let ptr_value = func_data.dfg_mut().new_value().load(param_ptr);
                func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(ptr_value).unwrap();
                ptr_value
            }
            
            // 非数组类型：回退到普通的 load 操作
            _ => self.generate_lval_load(lval)
        }
    }

    // 辅助方法：尝试从表达式中提取 LVal - 修复生命周期问题
    fn try_extract_lval(&self, exp: &Exp) -> Option<LVal> {
        // 需要层层解析 Exp -> LOrExp -> LAndExp -> EqExp -> RelExp -> AddExp -> MulExp -> UnaryExp -> PrimaryExp -> LVal
        let Exp::LOr(lor_exp) = exp;
        if let LOrExp::LAnd(land_exp) = lor_exp.as_ref() {
            if let LAndExp::Eq(eq_exp) = land_exp.as_ref() {
                if let EqExp::Rel(rel_exp) = eq_exp.as_ref() {
                    if let RelExp::Add(add_exp) = rel_exp.as_ref() {
                        if let AddExp::Mul(mul_exp) = add_exp.as_ref() {
                            if let MulExp::Unary(unary_exp) = mul_exp.as_ref() {
                                if let UnaryExp::Primary(primary_exp) = unary_exp.as_ref() {
                                    if let PrimaryExp::LVal(lval) = primary_exp {
                                        return Some(lval.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}