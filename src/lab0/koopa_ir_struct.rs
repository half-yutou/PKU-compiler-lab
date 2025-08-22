use koopa::ir::{FunctionData, Program, Type};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};

/// 生成一个简单的koopa ir程序
pub fn gen_koopa_ir_in_memory() -> Program {
    let mut program = Program::new();
    // program 由若干 全局变量Value 和 若干 函数Function 组成
    let main_func_handler = program.new_func(FunctionData::with_param_names(
        "@main".to_string(),
        vec![(Some("%arg1".to_string()), Type::get_i32()), (Some("%arg2".to_string()), Type::get_array(Type::get_i32(), 8))],
        Type::get_i32(),
    ));
    // new_func 返回的是函数的handler，用于后续操作
    // 如果只创建上述函数声明，则生成的koopaIR同样使用decl开头标识而不是fun开头
    // 且函数入参名称被省略
    
    // 使用Handler从Program中取出函数本身
    let main_func_data = program.func_mut(main_func_handler);
    // 创建一个基本块basic block,并返回basic block的handler
    let basic_block_handler = main_func_data.dfg_mut()
        .new_bb().basic_block(Some("%entry".to_string()));
    // 给函数layout 添加基本块
    main_func_data.layout_mut().bbs_mut().extend([basic_block_handler]);
    
    // 给基本块添加指令(Value)
    // 这个是新建一个本地变量，可以指定其类型,这里指定其类型为i32，返回对应的Handler
    let local_variable_handler = main_func_data.dfg_mut().new_value().integer(10);
    // 这个也是新建一个本地变量，指定其类型是"返回指令"，同样返回对应的Handler
    let return_inst_handler = main_func_data.dfg_mut().new_value().ret(Some(local_variable_handler));
    // 使用基本块的Handler拿到基本块本身(bb_mut(handler)),
    // 再使用基本块的insts_mut()方法拿到指令列表，
    // 最后使用push_key_back()方法将返回指令添加到指令列表末尾
    main_func_data.layout_mut().bb_mut(basic_block_handler).insts_mut().push_key_back(return_inst_handler).unwrap();
    program
}


// 输出koopa ir文本到指定文件
fn print_koopa_ir(koopa_ir_in_memory: Program) -> std::io::Result<()> {
    let mut generator = koopa::back::KoopaGenerator::new(Vec::new());
    generator.generate_on(&koopa_ir_in_memory)?;
    let koopa_ir_text = std::str::from_utf8(&generator.writer()).unwrap().to_string();
    println!("{}", koopa_ir_text);
    Ok(())
}

#[test]
fn test_gen_koopa_ir_in_memory() {
    let koopa_ir_in_memory = gen_koopa_ir_in_memory();
    print_koopa_ir(koopa_ir_in_memory).unwrap();
}
