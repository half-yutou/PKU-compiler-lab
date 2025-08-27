fn main() {
    lalrpop::process_root().unwrap();
}
/*
测试命令
docker run -it --rm -v ./:/root/compiler maxxing/compiler-dev autotest ${MODE} -s lv${LEVEL} /root/compiler
*/
