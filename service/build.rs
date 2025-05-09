use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../recursion/circuit", Default::default());
    build_program_with_args("../recursion/wrapper-circuit", Default::default());
}
