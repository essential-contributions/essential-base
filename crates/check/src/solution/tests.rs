use super::*;
use crate::vm::asm;
use asm::short::*;
use essential_types::predicate::{Edge, Node};

type ContentAddr = u8;

fn p(i: &[(ContentAddr, &[Edge])]) -> Arc<Predicate> {
    let mut nodes = vec![];
    let mut all_edges = vec![];
    let mut last_edge_start = 0;
    for (n, edges) in i {
        let node = Node {
            edge_start: if edges.is_empty() {
                Edge::MAX
            } else {
                last_edge_start
            },
            program_address: ContentAddress([*n; 32]),
        };
        last_edge_start += edges.len() as u16;
        nodes.push(node);
        all_edges.extend_from_slice(edges);
    }
    Arc::new(Predicate {
        nodes,
        edges: all_edges,
    })
}

type HasPost = bool;

fn get_p(progs: &[(ContentAddr, HasPost)]) -> HashMap<ContentAddress, Arc<Program>> {
    let mut progs_map = HashMap::new();
    for (addr, has_post) in progs {
        let program = if *has_post {
            Program(asm::to_bytes([PKRNG]).collect())
        } else {
            Program(asm::to_bytes([POP]).collect())
        };
        progs_map.insert(ContentAddress([*addr; 32]), Arc::new(program));
    }
    progs_map
}

type NodeIx = u16;
type StackV = Word;
type MemV = Word;

fn c(i: &[(NodeIx, &[StackV], &[MemV])]) -> Cache {
    let mut cache = Cache::default();
    for (node, stack, mem) in i {
        cache.insert(*node, parent(stack, mem));
    }
    cache
}

fn parent(stack: &[StackV], mem: &[MemV]) -> Arc<(Stack, Memory)> {
    let mut s = Stack::default();
    let mut m = Memory::default();
    for v in stack {
        s.push(*v).unwrap();
    }
    m.alloc(mem.len() as i64).unwrap();
    for (i, v) in mem.iter().enumerate() {
        m.store(i as i64, *v).unwrap();
    }
    Arc::new((s, m))
}

fn make_mem(i: &[MemV]) -> Memory {
    let mut m = Memory::default();
    m.alloc(i.len() as i64).unwrap();
    for (i, v) in i.iter().enumerate() {
        m.store(i as i64, *v).unwrap();
    }
    m
}

#[test]
fn test_check_predicate_inner() {
    // Sanity
    let predicate = p(&[(0, &[1, 2]), (1, &[2]), (2, &[])]);
    let get_program = get_p(&[(0, false), (1, false), (2, false)]);
    let mut cache = c(&[]);
    let ctx = Ctx {
        run_mode: RunMode::Outputs,
        cache: &mut cache,
    };
    let run = |ix, _| {
        let o = match ix {
            0 => Output::Parent(parent(&[], &[])),
            1 => Output::Parent(parent(&[], &[])),
            2 => Output::Leaf(ProgramOutput::Satisfied(true)),
            _ => unreachable!(),
        };
        (ix, Ok::<_, ProgramError<String>>((o, 0)))
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert!(out.is_empty());
    assert!(cache.is_empty());

    // Output on run output
    let ctx = Ctx {
        run_mode: RunMode::Outputs,
        cache: &mut cache,
    };
    let run = |ix, _| {
        let o = match ix {
            0 => Output::Parent(parent(&[], &[])),
            1 => Output::Parent(parent(&[], &[])),
            2 => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(make_mem(&[
                1, 2,
            ])))),
            _ => unreachable!(),
        };
        (ix, Ok::<_, ProgramError<String>>((o, 0)))
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], DataOutput::Memory(make_mem(&[1, 2])));
    assert!(cache.is_empty());

    // Nothing on checks run
    let ctx = Ctx {
        run_mode: RunMode::Checks,
        cache: &mut cache,
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert!(out.is_empty());
    assert!(cache.is_empty());

    // False constraint fails on outputs run
    let ctx = Ctx {
        run_mode: RunMode::Outputs,
        cache: &mut cache,
    };
    let run = |ix, _| {
        let o = match ix {
            0 => Output::Parent(parent(&[], &[])),
            1 => Output::Parent(parent(&[], &[])),
            2 => Output::Leaf(ProgramOutput::Satisfied(false)),
            _ => unreachable!(),
        };
        (ix, Ok::<_, ProgramError<String>>((o, 0)))
    };
    check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap_err();
    assert!(cache.is_empty());

    // False constraint is filtered out on checks run
    let ctx = Ctx {
        run_mode: RunMode::Checks,
        cache: &mut cache,
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert!(out.is_empty());
    assert!(cache.is_empty());

    //   0
    //  / \
    // 1   2
    // |\  |
    // 5 3 *4
    //    \|
    //     6
    // 2 and 3 should cache
    // 4 and 6 shouldn't run
    let predicate = p(&[
        (0, &[1, 2]),
        (1, &[5, 3]),
        (2, &[4]),
        (3, &[6]),
        (4, &[6]),
        (5, &[]),
        (6, &[]),
    ]);
    let get_program = get_p(&[
        (0, false),
        (1, false),
        (2, false),
        (3, false),
        (4, true),
        (5, false),
        (6, false),
    ]);

    let mut cache = c(&[]);
    let ctx = Ctx {
        run_mode: RunMode::Outputs,
        cache: &mut cache,
    };
    let run = |ix, inputs: Vec<Arc<(Stack, Memory)>>| {
        match ix {
            0 => assert!(inputs.is_empty()),
            1 => {
                assert_eq!(inputs.len(), 1);
                assert_eq!(inputs[0], parent(&[], &[0]));
            }
            2 => {
                assert_eq!(inputs.len(), 1);
                assert_eq!(inputs[0], parent(&[], &[0]));
            }
            3 => {
                assert_eq!(inputs.len(), 1);
                assert_eq!(inputs[0], parent(&[], &[1]));
            }
            5 => {
                assert_eq!(inputs.len(), 1);
                assert_eq!(inputs[0], parent(&[], &[1]));
            }
            prog => panic!("Ran unexpected program {}", prog),
        }
        let o = match ix {
            0 => Output::Parent(parent(&[], &[0])),
            1 => Output::Parent(parent(&[], &[1])),
            2 => Output::Parent(parent(&[], &[2])),
            3 => Output::Parent(parent(&[], &[3])),
            4 => Output::Parent(parent(&[], &[4])),
            5 => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(make_mem(&[
                5,
            ])))),
            6 => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(make_mem(&[
                6,
            ])))),
            _ => unreachable!(),
        };
        (ix, Ok::<_, ProgramError<String>>((o, 0)))
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], DataOutput::Memory(make_mem(&[5])));
    assert_eq!(cache.len(), 2);
    assert_eq!(cache[&2], parent(&[], &[2]));
    assert_eq!(cache[&3], parent(&[], &[3]));

    let ctx = Ctx {
        run_mode: RunMode::Checks,
        cache: &mut cache,
    };
    let run = |ix, inputs: Vec<Arc<(Stack, Memory)>>| {
        match ix {
            4 => {
                assert_eq!(inputs.len(), 1);
                assert_eq!(inputs[0], parent(&[], &[2]));
            }
            6 => {
                assert_eq!(inputs.len(), 2);
                assert_eq!(inputs[0], parent(&[], &[3]));
                assert_eq!(inputs[1], parent(&[], &[4]));
            }
            prog => panic!("Ran unexpected program {}", prog),
        }
        let o = match ix {
            0 => Output::Parent(parent(&[], &[0])),
            1 => Output::Parent(parent(&[], &[1])),
            2 => Output::Parent(parent(&[], &[2])),
            3 => Output::Parent(parent(&[], &[3])),
            4 => Output::Parent(parent(&[], &[4])),
            5 => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(make_mem(&[
                5,
            ])))),
            6 => Output::Leaf(ProgramOutput::DataOutput(DataOutput::Memory(make_mem(&[
                6,
            ])))),
            _ => unreachable!(),
        };
        (ix, Ok::<_, ProgramError<String>>((o, 0)))
    };
    let (_, out) = check_predicate_inner(
        run,
        predicate.clone(),
        &Default::default(),
        &get_program,
        ctx,
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], DataOutput::Memory(make_mem(&[6])));

    assert_eq!(cache.len(), 2);
}
