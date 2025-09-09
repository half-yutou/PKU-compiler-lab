use std::collections::HashMap;
use koopa::ir::Value;

/// 符号信息：区分常量、局部变量和全局变量
#[derive(Debug, Clone)]
pub enum SymbolInfo {
    Const(i32),           // 常量：直接存储值
    Var(Value),           // 局部变量：存储 alloc 返回的指针
    GlobalVar(Value),     // 全局变量：存储 global_alloc 返回的指针
}

// 作用域栈：支持嵌套作用域的符号表
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<HashMap<String, SymbolInfo>>,  // 作用域栈，每层是一个符号表
    var_counter: HashMap<String, usize>,       // 变量重命名计数器
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // 初始化全局符号表
            var_counter: HashMap::new(),
        }
    }
    
    // 进入新作用域，压栈一个符号表
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    // 退出当前作用域，出栈一个符号表
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    
    // 在当前定义域定义符号
    pub fn define(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        if let Some(current_scope) = self.scopes.last_mut() {
            if current_scope.contains_key(&name) {
                return Err(format!("Symbol '{}' already defined in current scope", name))
            }
            current_scope.insert(name, info);
            Ok(())
        } else {
            Err("No active scope".to_string())
        }
    }
    
    // 在全局作用域定义符号（用于全局变量和常量）
    pub fn define_global(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
        if let Some(global_scope) = self.scopes.first_mut() {
            if global_scope.contains_key(&name) {
                return Err(format!("Global symbol '{}' already defined", name))
            }
            global_scope.insert(name, info);
            Ok(())
        } else {
            Err("No global scope available".to_string())
        }
    }
    
    // 从内层向外层作用域查找符号
    pub fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info)
            }
        }
        None
    }
    
    /// 生成唯一的变量名(用于koopaIR)
    /// 标识不同作用域的同名符号
    /// ```c
    /// int a = 0;      // @a_1
    /// {
    ///     int a = 1;  // @a_2
    /// }
    /// ```
    pub fn generate_unique_name(&mut self, base_name: &str) -> String {
        let counter = self.var_counter.entry(base_name.to_string()).or_insert(0);
        *counter += 1;
        format!("@{}_{}", base_name, counter)
    }
}










