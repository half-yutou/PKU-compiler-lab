fn main() {
    lalrpop::process_root().unwrap();
}
/*
本地运行命令
cargo run -- -koopa hello.c -o koopair.txt
cargo run -- -riscv hello.c -o riscv.txt

本地测试命令
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest ${MODE} -s lv${LEVEL} /root/compiler
*/
