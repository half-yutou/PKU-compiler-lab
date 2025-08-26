use koopa::ir::{BinaryOp, FunctionData, Program, Type, Value};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use crate::ast::{CompUnit, Exp, UnaryExp, PrimaryExp, UnaryOp, AddExp, MulExp, MulDivOp, PlusSubOp};

pub fn generate_koopa_ir(ast: CompUnit) -> Program {
    let mut program = Program::new();
    let main_func = program.new_func(FunctionData::with_param_names(
        "@main".into(),
        vec![],
        Type::get_i32(),
    ));
    let main_data = program.func_mut(main_func);

    let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    main_data.layout_mut().bbs_mut().extend([entry]);

    // 生成表达式对应的 Koopa IR 指令
    let result_value = generate_exp(&ast.func_def.block.stmt.exp, main_data);
    let ret = main_data.dfg_mut().new_value().ret(Some(result_value));
    main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();

    program
}

// 生成表达式的 Koopa IR
fn generate_exp(exp: &Exp, func_data: &mut FunctionData) -> Value {
    match exp {
        Exp::AddExp(add_exp) => generate_add_exp(add_exp, func_data),
    }
}

// 生成加法表达式的 Koopa IR(后序遍历，先处理左右节点，再处理中间节点)
fn generate_add_exp(add_exp: &AddExp, func_data: &mut FunctionData) -> Value {
    match add_exp {
        AddExp::Mul(mul_exp) => generate_mul_exp(mul_exp, func_data),
        AddExp::AddMul(left, op, right) => {
            let left_value = generate_add_exp(left, func_data);
            let right_value = generate_mul_exp(right, func_data);
            generate_add_binary_op(op, left_value, right_value, func_data)
        }
    }
}

// 生成乘法表达式的 Koopa IR(后续遍历， 先处理左右节点，再处理中间节点)
fn generate_mul_exp(mul_exp: &MulExp, func_data: &mut FunctionData) -> Value {
    match mul_exp {
        MulExp::Unary(unary_exp) => generate_unary_exp(unary_exp, func_data),
        MulExp::MulDiv(left, op, right) => {
            let left_value = generate_mul_exp(left, func_data);
            let right_value = generate_unary_exp(right, func_data);
            generate_mul_binary_op(op, left_value, right_value, func_data)
        }
    }
}

fn generate_add_binary_op(op: &PlusSubOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        PlusSubOp::Plus => BinaryOp::Add,
        PlusSubOp::Minus => BinaryOp::Sub,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

fn generate_mul_binary_op(op: &MulDivOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        MulDivOp::Mul => BinaryOp::Mul,
        MulDivOp::Div => BinaryOp::Div,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

// 生成一元表达式的 Koopa IR
fn generate_unary_exp(unary_exp: &UnaryExp, func_data: &mut FunctionData) -> Value {
    match unary_exp {
        UnaryExp::Primary(primary) => generate_primary_exp(primary, func_data),
        UnaryExp::Unary(op, exp) => {
            let operand = generate_unary_exp(exp, func_data);
            generate_unary_op(op, operand, func_data)
        }
    }
}

// 生成基本表达式的 Koopa IR
fn generate_primary_exp(primary: &PrimaryExp, func_data: &mut FunctionData) -> Value {
    match primary {
        PrimaryExp::Number(num) => {
            func_data.dfg_mut().new_value().integer(*num)
        },
        PrimaryExp::Paren(exp) => generate_exp(exp, func_data),
    }
}

// 生成一元运算符的 Koopa IR
fn generate_unary_op(op: &UnaryOp, operand: Value, func_data: &mut FunctionData) -> Value {
    match op {
        UnaryOp::Plus => {
            // + 运算符不生成任何指令，直接返回操作数
            operand
        },
        UnaryOp::Minus => {
            // - 运算符：0 - operand
            let zero = func_data.dfg_mut().new_value().integer(0);
            let sub_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Sub, zero, operand);
            // 将指令添加到当前基本块
            let entry = func_data.layout().entry_bb().unwrap();
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(sub_inst).unwrap();
            sub_inst
        },
        UnaryOp::Not => {
            // ! 运算符：operand == 0
            let zero = func_data.dfg_mut().new_value().integer(0);
            let eq_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Eq, operand, zero);
            // 将指令添加到当前基本块
            let entry = func_data.layout().entry_bb().unwrap();
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(eq_inst).unwrap();
            eq_inst
        }
    }
}