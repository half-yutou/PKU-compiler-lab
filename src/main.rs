use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::read_to_string;
use std::io::Result;

// 引用 lalrpop 生成的解析器
// 因为我们刚刚创建了 sysy.lalrpop, 所以模块名是 sysy
lalrpop_mod!(sysy);

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

    // 输出解析得到的 AST
    // println!("{:#?}", ast);
    // println!("============");
    // 上述ast是内存中的数据结构，我们需要将其转换为koopa ir内存形式
    let koopa_ir_in_memory = pku_compiler::generate_koopa_ir(ast);
    // let koopa_ir_in_memory = pku_compiler::fib_koopa_ir();
    // 转换为文本形式
    let mut generator = koopa::back::KoopaGenerator::new(Vec::new());
    generator.generate_on(&koopa_ir_in_memory)?;
    let koopa_ir_text = std::str::from_utf8(&generator.writer()).unwrap().to_string();
    // 输出koopa ir文本
    println!("{}", koopa_ir_text);
    // 写入输出文件
    std::fs::write(output, koopa_ir_text)?;

    Ok(())
}
