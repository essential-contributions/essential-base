use super::*;
use crate::vm::asm;
use asm::short::*;

#[tokio::test]
async fn test_program_output() {
    // Satisfied true
    let p = Program(asm::to_bytes([PUSH(1)]).collect());
    let (o, _) = run_program(
        PanicStateRead,
        PanicStateRead,
        empty_sol_set(),
        0,
        Arc::new(p),
        empty_prgm_ctx(),
    )
    .await
    .unwrap();
    assert_eq!(o, Some(ProgramOutput::Satisfied(true)));

    // Satisfied false 0
    let p = Program(asm::to_bytes([PUSH(0)]).collect());
    let (o, _) = run_program(
        PanicStateRead,
        PanicStateRead,
        empty_sol_set(),
        0,
        Arc::new(p),
        empty_prgm_ctx(),
    )
    .await
    .unwrap();
    assert_eq!(o, Some(ProgramOutput::Satisfied(false)));

    // Satisfied false 3
    let p = Program(asm::to_bytes([PUSH(3)]).collect());
    let (o, _) = run_program(
        PanicStateRead,
        PanicStateRead,
        empty_sol_set(),
        0,
        Arc::new(p),
        empty_prgm_ctx(),
    )
    .await
    .unwrap();
    assert_eq!(o, Some(ProgramOutput::Satisfied(false)));

    // Memory
    let p = Program(
        asm::to_bytes([PUSH(42), PUSH(43), PUSH(2), PUSH(2), ALOC, STOR, PUSH(2)]).collect(),
    );
    let (o, _) = run_program(
        PanicStateRead,
        PanicStateRead,
        empty_sol_set(),
        0,
        Arc::new(p),
        empty_prgm_ctx(),
    )
    .await
    .unwrap();
    let mem = vec![42, 43].try_into().unwrap();
    assert_eq!(o, Some(ProgramOutput::DataOutput(DataOutput::Memory(mem))));
}

fn empty_sol_set() -> Arc<SolutionSet> {
    Arc::new(SolutionSet {
        solutions: vec![Solution {
            predicate_to_solve: PredicateAddress {
                contract: ContentAddress([0; 32]),
                predicate: ContentAddress([0; 32]),
            },
            predicate_data: vec![],
            state_mutations: vec![],
        }],
    })
}

fn empty_prgm_ctx() -> ProgramCtx {
    ProgramCtx {
        parents: vec![],
        children: vec![],
        reads: Default::default(),
    }
}

struct PanicStateRead;

impl StateRead for PanicStateRead {
    type Error = String;

    type Future = std::future::Ready<Result<Vec<Vec<Word>>, Self::Error>>;

    fn key_range(
        &self,
        _contract_addr: ContentAddress,
        _key: Key,
        _num_values: usize,
    ) -> Self::Future {
        panic!("StateRead::key_range called")
    }
}
