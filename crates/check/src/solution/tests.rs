use super::*;
use essential_types::predicate::Edge;

#[test]
fn test_check_predicate_sync_inner() {
    let predicate = Predicate {
        nodes: vec![],
        edges: vec![],
    };
    let predicate = Arc::new(predicate);
    let run = |(ix, _): (&u16, &_)| {
        (
            *ix,
            Result::<_, ProgramError<String>>::Ok((
                Output::Leaf(ProgramOutput::Satisfied(true)),
                0,
            )),
        )
    };
    let res = check_predicate_sync_inner(run, predicate, &Default::default());
    assert_eq!((0, vec![]), res.unwrap());

    let predicate = Predicate {
        nodes: vec![essential_types::predicate::Node {
            edge_start: Edge::MAX,
            program_address: ContentAddress([0; 32]),
        }],
        edges: vec![],
    };
    let predicate = Arc::new(predicate);
    let run = |(ix, _): (&u16, &_)| {
        (
            *ix,
            Result::<_, ProgramError<String>>::Ok((
                Output::Leaf(ProgramOutput::Satisfied(true)),
                0,
            )),
        )
    };
    let res = check_predicate_sync_inner(run, predicate, &Default::default());
    assert_eq!((0, vec![]), res.unwrap());

    let predicate = Predicate {
        nodes: vec![essential_types::predicate::Node {
            edge_start: Edge::MAX,
            program_address: ContentAddress([0; 32]),
        }],
        edges: vec![],
    };
    let predicate = Arc::new(predicate);
    let run = |(ix, _): (&u16, &_)| {
        (
            *ix,
            Result::<_, ProgramError<String>>::Ok((
                Output::Parent(Arc::new((Default::default(), Default::default()))),
                0,
            )),
        )
    };
    let res = check_predicate_sync_inner(run, predicate, &Default::default());
    assert_eq!((0, vec![]), res.unwrap());
}
