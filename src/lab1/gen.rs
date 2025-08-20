use koopa::ir::{BinaryOp, FunctionData, Program, Type};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use crate::CompUnit;

pub fn generate_koopa_ir(ast: CompUnit) -> Program {
    let mut program = Program::new();
    // 为程序添加一个函数，返回其函数标识符
    let main_func = program.new_func(FunctionData::with_param_names(
        "@main".into(),
        vec![],
        Type::get_i32(),
    ));
    // 通过函数标识符获取函数元数据，使得可以对函数进行修改
    let main_data = program.func_mut(main_func);

    // 为函数添加一个基本块
    let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    main_data.layout_mut().bbs_mut().extend([entry]);

    // 为entry这个基本块添加一个返回指令
    let ret_val = main_data.dfg_mut().new_value().integer(ast.func_def.block.stmt.num);
    let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
    main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();

    program
}

pub fn fib_koopa_ir() -> Program {
    // create program and function
    let mut program = Program::new();
    let fib = program.new_func(FunctionData::with_param_names(
        "@fib".into(),
        vec![(Some("@n".into()), Type::get_i32())],
        Type::get_i32(),
    ));
    let fib_data = program.func_mut(fib);
    let n = fib_data.params()[0];

    // entry/then/else basic block
    let entry = fib_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    let then = fib_data.dfg_mut().new_bb().basic_block(Some("%then".into()));
    let else_bb = fib_data.dfg_mut().new_bb().basic_block(Some("%else".into()));
    fib_data.layout_mut().bbs_mut().extend([entry, then, else_bb]);

    // instructions in entry basic block
    let two = fib_data.dfg_mut().new_value().integer(2);
    let cond = fib_data.dfg_mut().new_value().binary(BinaryOp::Le, n, two);
    let br = fib_data.dfg_mut().new_value().branch(cond, then, else_bb);
    fib_data.layout_mut().bb_mut(entry).insts_mut().extend([cond, br]);

    // instructions in `then` basic block
    let one = fib_data.dfg_mut().new_value().integer(1);
    let ret = fib_data.dfg_mut().new_value().ret(Some(one));
    fib_data.layout_mut().bb_mut(then).insts_mut().push_key_back(ret).unwrap();

    // instructions in `else` basic block
    let sub1 = fib_data.dfg_mut().new_value().binary(BinaryOp::Sub, n, one);
    let call1 = fib_data.dfg_mut().new_value().call(fib, vec![sub1]);
    let sub2 = fib_data.dfg_mut().new_value().binary(BinaryOp::Sub, n, two);
    let call2 = fib_data.dfg_mut().new_value().call(fib, vec![sub2]);
    let ans = fib_data.dfg_mut().new_value().binary(BinaryOp::Add, call1, call2);
    let ret = fib_data.dfg_mut().new_value().ret(Some(ans));
    fib_data.layout_mut().bb_mut(else_bb).insts_mut().extend([sub1, call1, sub2, call2, ans, ret]);
    program
}