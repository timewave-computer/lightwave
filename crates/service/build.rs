use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../integrations/sp1-helios/circuit", Default::default());
    build_program_with_args(
        "../integrations/sp1-helios/wrapper-circuit",
        Default::default(),
    );
    build_program_with_args("../integrations/sp1-tendermint/circuit", Default::default());
    build_program_with_args(
        "../integrations/sp1-tendermint/wrapper-circuit",
        Default::default(),
    );
}
