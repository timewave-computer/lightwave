use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../sp1-helios/circuit", Default::default());
    build_program_with_args("../sp1-helios/wrapper-circuit", Default::default());
    build_program_with_args("../sp1-tendermint/circuit", Default::default());
    build_program_with_args("../sp1-tendermint/wrapper-circuit", Default::default());
}
