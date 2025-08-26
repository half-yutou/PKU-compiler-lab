// use crate::ast::{CompUnit, Exp, PrimaryExp, UnaryExp, UnaryOp};
// use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
// use koopa::ir::{FunctionData, Program, Type};
// 
// // 新增：计算表达式值的函数
// fn evaluate_exp(exp: &Exp) -> i32 {
//     match exp {
//         Exp::Unary(unary_exp) => evaluate_unary_exp(unary_exp),
//     }
// }
// 
// fn evaluate_unary_exp(unary_exp: &UnaryExp) -> i32 {
//     match unary_exp {
//         UnaryExp::Primary(primary) => evaluate_primary_exp(primary),
//         UnaryExp::Unary(op, exp) => {
//             let val = evaluate_unary_exp(exp);
//             match op {
//                 UnaryOp::Plus => val,
//                 UnaryOp::Minus => -val,
//                 UnaryOp::Not => if val == 0 { 1 } else { 0 },
//             }
//         }
//     }
// }
// 
// fn evaluate_primary_exp(primary: &PrimaryExp) -> i32 {
//     match primary {
//         PrimaryExp::Number(num) => *num,
//         PrimaryExp::Paren(exp) => evaluate_exp(exp),
//     }
// }
// 
// pub fn generate_koopa_ir(ast: CompUnit) -> Program {
//     let mut program = Program::new();
//     let main_func = program.new_func(FunctionData::with_param_names(
//         "@main".into(),
//         vec![],
//         Type::get_i32(),
//     ));
//     let main_data = program.func_mut(main_func);
// 
//     let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
//     main_data.layout_mut().bbs_mut().extend([entry]);
// 
//     // 修改这里：使用新的表达式计算函数
//     let result_value = evaluate_exp(&ast.func_def.block.stmt.exp);
//     let ret_val = main_data.dfg_mut().new_value().integer(result_value);
//     let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
//     main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
// 
//     program
// }