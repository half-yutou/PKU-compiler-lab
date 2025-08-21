use koopa::ir::Program;
use lalrpop_util::lalrpop_mod;
use pku_compiler::lab3;
use std::env::args;
use std::fs::read_to_string;
use std::io::Result;

// 引用 lalrpop 生成的解析器
// 因为我们刚刚创建了 sysy.lalrpop, 所以模块名是 sysy
lalrpop_mod!(sysy);

const MODE_KOOPA: &str = "-koopa";
const MODE_RISCV: &str = "-riscv";

fn main() -> Result<()> {
    // 解析命令行参数
    let mut args = args();
    args.next();
    let mode = args.next().unwrap();
    let input = args.next().unwrap();
    args.next();
    let output = args.next().unwrap();

    // 读取输入文件
    let input = read_to_string(input)?;

    // 调用 lalrpop 生成的 parser 解析输入文件
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();
    let koopa_ir_in_memory = lab3::gen::generate_koopa_ir(ast);

    if mode == MODE_KOOPA {
        output_koopa_ir(koopa_ir_in_memory, &output)?;
    } else if mode == MODE_RISCV {
        output_riscv_assembly(koopa_ir_in_memory, &output)?;
    } else {
        panic!("invalid mode");
    }

    Ok(())
}

// 输出koopa ir文本到指定文件
fn output_koopa_ir(koopa_ir_in_memory: Program, output_file: &str) -> Result<()> {
    let mut generator = koopa::back::KoopaGenerator::new(Vec::new());
    generator.generate_on(&koopa_ir_in_memory)?;
    let koopa_ir_text = std::str::from_utf8(&generator.writer()).unwrap().to_string();
    std::fs::write(output_file, koopa_ir_text)?;
    Ok(())
}

// 输出risc-v汇编到指定文件
fn output_riscv_assembly(koopa_ir_in_memory: Program, output_file: &str) -> Result<()> {
    let riscv_assembly_text = pku_compiler::lab2::gen::generate_riskv_assembly(koopa_ir_in_memory);
    std::fs::write(output_file, riscv_assembly_text)?;
    Ok(())
}
