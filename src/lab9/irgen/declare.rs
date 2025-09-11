use crate::ast::{ConstInitVal, Decl, InitVal};
use crate::lab9::irgen::symbol::SymbolInfo;
use crate::lab9::irgen::IRGen;
use koopa::ir::builder::LocalInstBuilder;
use koopa::ir::Type;

// 处理声明(常量与变量)
impl IRGen {
    pub fn generate_decl(&mut self, decl: &Decl) {
        match decl {
            // const int a = 1, b = 2 + 3, c = (a > b);
            Decl::Const(const_decl) => {
                for def in &const_decl.const_def_list {
                    if def.dimensions.is_empty() {
                        // 普通常量
                        let value = match &def.const_init_val {
                            ConstInitVal::Exp(const_exp) => {
                                self.evaluate_lor_exp(&const_exp.lor_exp)
                            }
                            ConstInitVal::List(_) => {
                                panic!("Scalar constant cannot have list initializer")
                            }
                        };
                        
                        // 检查是否重复定义并存入符号表
                        if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
                            panic!("{}", err)
                        }
                    } else {
                        // 常量数组 - 与变量数组采用相同的处理方式
                        let unique_name = self.function_irgen.scope_stack.generate_unique_name(&def.ident);

                        let mut dimensions = Vec::new();
                        for dim_exp in &def.dimensions {
                            let dim_value = self.evaluate_lor_exp(&dim_exp.lor_exp);
                            dimensions.push(dim_value as usize);
                        }

                        // 创建数组类型
                        let mut array_type = Type::get_i32();
                        for &dim in dimensions.iter().rev() {
                            array_type = Type::get_array(array_type, dim);
                        }

                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();

                        // 为数组分配内存
                        let alloc_inst = func_data.dfg_mut().new_value().alloc(array_type.clone());
                        func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(alloc_inst).unwrap();

                        // 处理初始化值
                        use crate::lab9::irgen::array::LocalInitializer;
                        
                        let initializer = LocalInitializer::from_const_init_val(&def.const_init_val, self)
                            .unwrap_or_else(|e| panic!("Failed to create local initializer: {}", e));
                        let reshaped = initializer.reshape(&array_type)
                            .unwrap_or_else(|e| panic!("Failed to reshape initializer: {}", e));
                        let init_values = reshaped.flatten(self);
                        
                        // 使用 getelemptr 和 store 指令初始化数组
                        self.initialize_local_array(alloc_inst, &init_values, &dimensions);
                        
                        // 存入符号表 - 存储指针和维度信息
                        if let Err(err) = self.function_irgen.scope_stack.define(
                            def.ident.clone(), 
                            SymbolInfo::LocalConstArray(alloc_inst, dimensions)
                        ) {
                            panic!("{}", err)
                        }
                    }
                }
            }
            
            // int a, b = 2 + 3, c, d = (a > b) || (c != 0);
            Decl::Var(var_decl) => {
                for def in &var_decl.var_def_list {
                    let unique_name = self.function_irgen.scope_stack.generate_unique_name(&def.ident);

                    if def.dimensions.is_empty() {
                        // 普通变量
                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();

                        // 为变量分配内存(简单起见，全部分配到栈上-无寄存器分配策略)
                        let alloc_ptr = func_data.dfg_mut().new_value().alloc(Type::get_i32());// 返回这个变量的指针
                        func_data.dfg_mut().set_value_name(alloc_ptr, Some(unique_name));
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(alloc_ptr).unwrap();
                        
                        // 如果有初始化值，生成store指令
                        if let Some(init_val) = &def.init_val {
                            let init_value = match init_val {
                                InitVal::Exp(exp) => {
                                    self.generate_exp(exp)
                                }
                                InitVal::List(_) => {
                                    panic!("Scalar variable cannot have list initializer")
                                }
                            };
                            
                            let current_bb = self.current_bb();// 重新获取current_bb, generate_exp()可能更改了current_bb
                            let func_data = self.function_data_mut();
                            
                            let store_inst = func_data.dfg_mut().new_value().store(init_value, alloc_ptr);
                            func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(store_inst).unwrap()
                        }
                        
                        // 检查重复定义并存入符号表
                        if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Var(alloc_ptr)) {
                            panic!("{}", err)
                        }
                    } else {
                        // 数组变量 - 使用 getelemptr 和 store 指令初始化
                        let mut dimensions = Vec::new();
                        for dim_exp in &def.dimensions {
                            let dim_value = self.evaluate_lor_exp(&dim_exp.lor_exp);
                            dimensions.push(dim_value as usize);
                        }
                        
                        // 创建数组类型
                        let mut array_type = Type::get_i32();
                        for &dim in dimensions.iter().rev() {
                            array_type = Type::get_array(array_type, dim);
                        }
                        
                        let current_bb = self.current_bb();
                        let func_data = self.function_data_mut();
                        
                        // 为数组分配内存
                        let alloc_inst = func_data.dfg_mut().new_value().alloc(array_type.clone());
                        func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
                        func_data.layout_mut().bb_mut(current_bb).insts_mut().push_key_back(alloc_inst).unwrap();
                        
                        // 处理初始化值
                        if let Some(init_val) = &def.init_val {
                            use crate::lab9::irgen::array::LocalInitializer;
                            
                            let initializer = LocalInitializer::from_init_val(init_val, self)
                                .unwrap_or_else(|e| panic!("Failed to create local initializer: {}", e));
                            let reshaped = initializer.reshape(&array_type)
                                .unwrap_or_else(|e| panic!("Failed to reshape initializer: {}", e));
                            let init_values = reshaped.flatten(self);
                            
                            // 使用 getelemptr 和 store 指令初始化数组
                            self.initialize_local_array(alloc_inst, &init_values, &dimensions);
                        }
                        
                        // 存入符号表
                        if let Err(err) = self.function_irgen.scope_stack.define(
                            def.ident.clone(), 
                            SymbolInfo::LocalArray(alloc_inst, dimensions)
                        ) {
                            panic!("{}", err)
                        }
                    }
                }
            }
        }
    }
}

