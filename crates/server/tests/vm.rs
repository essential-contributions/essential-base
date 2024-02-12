use essential_types::solution::KeyMutation;
use essential_types::solution::Mutation;
use essential_types::solution::RangeMutation;
use essential_types::solution::Sender;
use essential_types::solution::SolutionData;
use essential_types::solution::StateMutation;
use essential_types::SourceAddress;
use intent_server::check::pack_n_bytes;
use intent_server::check::Directive;
use intent_server::data::Slots;
use intent_server::db::add_to_key;
use intent_server::hash_words;
use intent_server::intent::Intent;
use intent_server::intent::ToIntentAddress;
use intent_server::solution::Solution;
use intent_server::state_read::StateSlot;
use intent_server::Server;
use state_asm::constraint_asm::*;
use state_asm::*;

fn eoa_sender() -> Sender {
    Sender::Eoa([0; 4])
}

#[test]
fn vm_state_reads() {
    let get_42 = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = serde_json::to_vec(&get_42).unwrap();
    let state_read = vec![state_read];

    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            state: vec![StateSlot {
                index: 0,
                amount: 1,
                program_index: 0,
            }],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();
    let deploy_address = server.deploy_intent_set(vec![intent.clone()]).unwrap();
    let source_address = SourceAddress::persistent(deploy_address.into(), intent.intent_address());

    let solution = Solution {
        state_mutations: Default::default(),
        data: [SolutionData {
            intent_to_solve: source_address,
            decision_variables: Default::default(),
            sender: eoa_sender(),
        }]
        .into_iter()
        .collect(),
    };

    server
        .db()
        .stage(deploy_address, [14, 14, 14, 14], Some(42));
    server.db().commit();

    let solution = server.check(solution).unwrap();
    assert_eq!(solution, 1);
}

// Extern state reads
#[test]
fn extern_state_reads() {
    let get_42 = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRangeExtern),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = serde_json::to_vec(&get_42).unwrap();
    let state_read = vec![state_read];

    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            state: vec![StateSlot {
                index: 0,
                amount: 1,
                program_index: 0,
            }],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let mut server = Server::new();
    let deploy_address = server.deploy_intent_set(vec![intent.clone()]).unwrap();
    let source_address = SourceAddress::persistent(deploy_address.into(), intent.intent_address());

    let solution = Solution {
        state_mutations: Default::default(),
        data: [SolutionData {
            intent_to_solve: source_address,
            decision_variables: Default::default(),
            sender: eoa_sender(),
        }]
        .into_iter()
        .collect(),
    };

    server.db().stage([1, 1, 1, 1], [14, 14, 14, 14], Some(42));
    server.db().commit();

    let solution = server.check(solution).unwrap();
    assert_eq!(solution, 1);
}

#[test]
fn cant_write_outside_reads() {
    let get_42 = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(14)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = serde_json::to_vec(&get_42).unwrap();
    let state_read = vec![state_read];

    let constraints = vec![
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(42),
        Op::Pred(Pred::Eq),
    ];
    let constraints = serde_json::to_vec(&constraints).unwrap();
    let constraints = vec![constraints];
    let intent = Intent {
        slots: Slots {
            state: vec![StateSlot {
                index: 0,
                amount: 1,
                program_index: 0,
            }],
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let intent_address = intent.address();

    let mut server = Server::new();
    let deploy_address = server.deploy_intent_set(vec![intent.clone()]).unwrap();
    let source_address = SourceAddress::persistent(deploy_address.into(), intent.intent_address());

    let solution = Solution {
        state_mutations: vec![StateMutation {
            address: intent_address.into(),
            mutations: vec![Mutation::Key(KeyMutation {
                key: [1, 1, 1, 1],
                value: None,
            })],
        }],
        data: [SolutionData {
            intent_to_solve: source_address,
            decision_variables: Default::default(),
            sender: eoa_sender(),
        }]
        .into_iter()
        .collect(),
    };

    server
        .db()
        .stage(intent_address, [14, 14, 14, 14], Some(42));
    server.db().commit();

    let error = server.check(solution);
    assert!(error.is_err());
}

#[test]
fn naughts_crosses() {
    let mut server = Server::new();

    // state: [Option<int>; 9] board = state.get(board_loc, 9);
    let get_board = vec![
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)),
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];

    // state: [Option<[int; 4]>; 9] player_moves = state.get(player_loc, 9 * 4);
    let get_player_moves = vec![
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Alu(Alu::Mul)),
        StateReadOp::Memory(Memory::Alloc),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)),
        StateReadOp::Constraint(Op::Push(9)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Alu(Alu::Mul)),
        StateReadOp::State(State::StateReadWordRange),
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let get_board = serde_json::to_vec(&get_board).unwrap();
    let get_player_moves = serde_json::to_vec(&get_player_moves).unwrap();
    let state_read = vec![get_board, get_player_moves];

    let mut constraints = vec![];

    // var board_loc: [int; 4];
    // constraint board_loc == sha256(concat("board", 0))
    let board = "board".as_bytes().to_vec();
    let board = pack_n_bytes(&board);
    let len = board.len() + 1;

    let hash_location = board
        .into_iter()
        .map(Op::Push)
        .chain([
            Op::Push(0),
            Op::Push(len as u64),
            Op::Crypto(Crypto::Sha256),
        ])
        .collect::<Vec<_>>();
    let mut constraint: Vec<_> = (0..4)
        .rev()
        .flat_map(|i| {
            [
                Op::Push(i),
                Op::Access(Access::DecisionVar),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Swap,
            ]
        })
        .collect();
    // Remove first and
    constraint.remove(3);
    // Remove last swap
    constraint.pop();
    let constraint = hash_location
        .clone()
        .into_iter()
        .chain(constraint)
        .collect::<Vec<_>>();
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // var player_loc: [int; 4] = sha256(concat("player", 0));
    let player = "player".as_bytes().to_vec();
    let player = pack_n_bytes(&player);
    let len = player.len() + 1;

    let player_location = player
        .into_iter()
        .map(Op::Push)
        .chain([
            Op::Push(0),
            Op::Push(len as u64),
            Op::Crypto(Crypto::Sha256),
        ])
        .collect::<Vec<_>>();
    let mut constraint: Vec<_> = (0..4)
        .rev()
        .flat_map(|i| {
            [
                Op::Push(4 + i),
                Op::Access(Access::DecisionVar),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Swap,
            ]
        })
        .collect();
    // Remove first and
    constraint.remove(3);
    // Remove last swap
    constraint.pop();
    let constraint = player_location
        .clone()
        .into_iter()
        .chain(constraint)
        .collect::<Vec<_>>();
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // constraint forall(board, |b| is_none(b') || (is_some(b') && b' == 0) || b' == 1)
    let mut constraint = (0..9)
        .flat_map(|i| {
            [
                // State must be none
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::StateIsSome),
                Op::Pred(Pred::Not),
                // Or 0
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::StateIsSome),
                Op::Push(0),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Pred(Pred::Or),
                // Or 1
                Op::Push(1),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::Or),
                Op::Pred(Pred::And),
            ]
        })
        .collect::<Vec<_>>();
    // Remove first And
    constraint.remove(20);
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // constraint forall(board, |b| is_none(b) || b == b')
    let mut constraint = (0..9)
        .flat_map(|i| {
            [
                // State before must be none
                Op::Push(i),
                Op::Push(0),
                Op::Access(Access::StateIsSome),
                Op::Pred(Pred::Not),
                // or it must have not changed
                Op::Push(i),
                Op::Push(0),
                Op::Access(Access::State),
                Op::Push(i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::Or),
                Op::Pred(Pred::And),
            ]
        })
        .collect::<Vec<_>>();
    // Remove first And
    constraint.remove(12);
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    // constraint forall(zip(board, player_moves), |(b, p)| (b != b' && is_none(p) && is_some(p')) || (b == b' && p == p'))
    let mut forall_len = 0;
    let mut constraint = (0..9)
        .flat_map(|i| {
            let forall = [
                // is_some(b) != is_some(b') || b != b'
                Op::Push(i),                     // [i]
                Op::Push(0),                     // [i, 0]
                Op::Access(Access::StateIsSome), // [is_some(b)]
                Op::Push(i),                     // [is_some(b), i]
                Op::Push(1),                     // [is_some(b), i, 1]
                Op::Access(Access::StateIsSome), // [is_some(b), is_some(b')]
                Op::Pred(Pred::Eq),              // [is_some(b) == is_some(b')]
                Op::Pred(Pred::Not),             // [is_some(b) != is_some(b')]
                Op::Push(i),                     // [is_some(b) != is_some(b'), i]
                Op::Push(0),                     // [is_some(b) != is_some(b'), i, 0]
                Op::Access(Access::State),       // [is_some(b) != is_some(b'), b]
                Op::Push(i),                     // [is_some(b) != is_some(b'), b, i]
                Op::Push(1),                     // [is_some(b) != is_some(b'), b, i, 1]
                Op::Access(Access::State),       // [is_some(b) != is_some(b'), b, b']
                Op::Pred(Pred::Eq),              // [is_some(b) != is_some(b'), b == b']
                Op::Pred(Pred::Not),             // [is_some(b) != is_some(b'), b != b']
                Op::Pred(Pred::Or),              // [is_some(b) != is_some(b') || b != b']
                // && is_none(p)
                Op::Push(i + 9), // [is_some(b) != is_some(b') || b != b', i + 9]
                Op::Push(0),     // [is_some(b) != is_some(b') || b != b', i + 9, 0]
                Op::Access(Access::StateIsSome), // [is_some(b) != is_some(b') || b != b', is_some(p)]
                Op::Pred(Pred::Not), // [is_some(b) != is_some(b') || b != b', is_none(p)]
                Op::Pred(Pred::And), // [is_some(b) != is_some(b') || b != b' && is_none(p)]
                // && is_some(p')
                Op::Push(i + 9), // [is_some(b) != is_some(b') || b != b' && is_none(p), i + 9]
                Op::Push(1),     // [is_some(b) != is_some(b') || b != b' && is_none(p), i + 9, 1]
                Op::Access(Access::StateIsSome), // [is_some(b) != is_some(b') || b != b' && is_none(p), is_some(p')]
                Op::Pred(Pred::And), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p')]
                // || (b == b' && p == p')
                // b == b'
                Op::Push(i), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), i]
                Op::Push(0), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), i, 0]
                Op::Access(Access::State), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b]
                Op::Push(i), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b, i]
                Op::Push(1), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b, i, 1]
                Op::Access(Access::State), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b, b']
                Op::Pred(Pred::Eq), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b']
                // && p == p'
                Op::Push(i + 9), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', i + 9]
                Op::Push(0), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', i + 9, 0]
                Op::Access(Access::State), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', p]
                Op::Push(i + 9), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', p, i + 9]
                Op::Push(1), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', p, i + 9, 1]
                Op::Access(Access::State), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', p, p']
                Op::Pred(Pred::Eq), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b', p == p']
                Op::Pred(Pred::And), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p'), b == b' && p == p']
                // ||
                Op::Pred(Pred::Or), // [is_some(b) != is_some(b') || b != b' && is_none(p) && is_some(p') || b == b' && p == p']
                Op::Pred(Pred::And),
            ];
            forall_len = forall.len();
            forall
        })
        .collect::<Vec<_>>();
    // Remove first And
    constraint.remove(forall_len - 1);
    let constraint = serde_json::to_vec(&constraint).unwrap();
    constraints.push(constraint);

    let deployed_intent = Intent {
        slots: Slots {
            state: vec![
                StateSlot {
                    index: 0,
                    amount: 9,
                    program_index: 0,
                },
                StateSlot {
                    index: 9,
                    amount: 9 * 4,
                    program_index: 1,
                },
            ],
            decision_variables: 8,
            ..Default::default()
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let game_intent_address = deployed_intent.address();

    let deployed_address = server.deploy_intent_set(vec![deployed_intent]).unwrap();

    let board = "board".as_bytes().to_vec();
    let mut board = pack_n_bytes(&board);
    board.push(0);
    let board_address = hash_words(&board);

    let player = "player".as_bytes().to_vec();
    let mut player = pack_n_bytes(&player);
    player.push(0);
    let player_address = hash_words(&player);

    // var pos: int = 2;
    // var move: int = 1;
    // var game_address: [int; 4] = ${deployed_address};
    // var board_pos_hash: [int; 4] = sha256(concat("board", 0));
    // var player_pos_hash: [int; 4] = sha256(concat("player", 0)).offset(4 * pos);
    //
    // state board_pos: Option<int> = state.extern.get(game_address, board_pos_hash, pos + 1).get(pos);
    // state player_pos: Option<[int; 4]> = state.extern.get(game_address, player_pos_hash, 4);
    //
    // constraint is_some(board_pos') && board_pos' == move;
    // constraint is_some(player_pos') && player_pos' == ${my_address};
    let mut state_read = vec![];
    let mut constraints = vec![];
    let add_constraint = |constraints: &mut Vec<_>, constraint| {
        let constraint = serde_json::to_vec(&constraint).unwrap();
        constraints.push(constraint);
    };

    // var pos: int = 2;
    let constraint = vec![
        Op::Push(0),
        Op::Access(Access::DecisionVar),
        Op::Push(2),
        Op::Pred(Pred::Eq),
    ];
    add_constraint(&mut constraints, constraint);
    // var move: int = 1;
    let constraint = vec![
        Op::Push(1),
        Op::Access(Access::DecisionVar),
        Op::Push(1),
        Op::Pred(Pred::Eq),
    ];
    add_constraint(&mut constraints, constraint);
    // var game_address: [int; 4] = ${deployed_address};
    let mut constraint: Vec<_> = (0..4)
        .flat_map(|i| {
            [
                Op::Push(2 + i),
                Op::Access(Access::DecisionVar),
                Op::Push(deployed_address[i as usize]),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
            ]
        })
        .collect();
    // Remove first and
    constraint.remove(4);
    add_constraint(&mut constraints, constraint);
    // var board_pos_hash: [int; 4] = sha256(concat("board", 0));
    let mut board_pos_hash = (0..4)
        .rev()
        .flat_map(|i| {
            [
                Op::Push(6 + i),
                Op::Access(Access::DecisionVar),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Swap,
            ]
        })
        .collect::<Vec<_>>();
    // Remove first and
    board_pos_hash.remove(3);
    // Remove last swap
    board_pos_hash.pop();
    let constraint = hash_location
        .clone()
        .into_iter()
        .chain(board_pos_hash)
        .collect();
    add_constraint(&mut constraints, constraint);
    // var player_pos_hash: [int; 4] = sha256(concat("player", 0)).offset(4 * pos);
    let mut player_pos_hash = (0..4)
        .rev()
        .flat_map(|i| {
            [
                Op::Push(10 + i),
                Op::Access(Access::DecisionVar),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Swap,
            ]
        })
        .collect::<Vec<_>>();
    // Remove first and
    player_pos_hash.remove(3);
    // Remove last swap
    player_pos_hash.pop();

    let offset_player = vec![
        // Pos
        Op::Push(0),
        Op::Access(Access::DecisionVar),
        Op::Push(4),
        Op::Alu(Alu::Mul),
        Op::Alu(Alu::HashOffset),
    ];

    let constraint = player_location
        .clone()
        .into_iter()
        .chain(offset_player)
        .chain(player_pos_hash)
        .collect();
    add_constraint(&mut constraints, constraint);

    // state board_pos: Option<int> = state.extern.get(game_address, board_pos_hash, pos + 1).get(pos);
    let read = vec![
        // Amount
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVar)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Alu(Alu::Add)), // [memory_amount]
        StateReadOp::Constraint(Op::Dup),           // [memory_amount, memory_amount]
        // Allocate memory
        StateReadOp::Memory(Memory::Alloc), // [memory_amount] // 5
        StateReadOp::Constraint(Op::Dup),   // [memory_amount, memory_amount]
        StateReadOp::Memory(Memory::Push),  // [memory_amount]
        // extern key
        StateReadOp::Constraint(Op::Push(2)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)), // [memory_amount, extern_key] // 10
        // board_pos_hash
        StateReadOp::Constraint(Op::Push(6)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)), // [memory_amount, extern_key, board_pos_hash]
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Memory(Memory::Load), // [memory_amount, extern_key, board_pos_hash, memory_amount] // 15
        StateReadOp::Constraint(Op::Push(0)),
        StateReadOp::Memory(Memory::Truncate),
        StateReadOp::State(State::StateReadWordRangeExtern), // [memory_amount, mem_address]
        StateReadOp::Constraint(Op::Swap),                   // [mem_address, memory_amount]
        // pos
        StateReadOp::Constraint(Op::Push(1)),       // 20
        StateReadOp::Constraint(Op::Alu(Alu::Sub)), // [mem_address, pos]
        StateReadOp::Constraint(Op::Alu(Alu::Add)), // [mem_pos]
        StateReadOp::Constraint(Op::Dup),           // [mem_pos, mem_pos]
        StateReadOp::Memory(Memory::IsSome),
        StateReadOp::Constraint(Op::Pred(Pred::Not)), // [mem_pos, is_none(mem_pos)] //  25
        // PC address to jump to
        StateReadOp::Constraint(Op::Push(35)),
        StateReadOp::Constraint(Op::Swap), // [mem_pos, jump_address, is_none(mem_pos)]
        // Jump if is none
        StateReadOp::ControlFlow(ControlFlow::JumpIf), // [mem_pos]
        // if is some
        // load the value
        StateReadOp::Memory(Memory::Load), // [value]
        // free the memory
        StateReadOp::Constraint(Op::Push(0)), // [value, 0] // 30
        StateReadOp::Memory(Memory::Truncate), // [value]
        // Put the value back in memory
        StateReadOp::Memory(Memory::Push), // []
        // PC address of halt
        StateReadOp::Constraint(Op::Push(38)),
        StateReadOp::ControlFlow(ControlFlow::Jump),
        // if is none
        StateReadOp::Constraint(Op::Push(0)), // 35
        StateReadOp::Memory(Memory::Truncate),
        // Push None to mem pos 0
        StateReadOp::Memory(Memory::PushNone),
        StateReadOp::ControlFlow(ControlFlow::Halt), // 38
    ];

    let read = serde_json::to_vec(&read).unwrap();
    state_read.push(read);

    // state player_pos: Option<[int; 4]> = state.extern.get(game_address, player_pos_hash, 4);
    let read = vec![
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Memory(Memory::Alloc),
        // extern key
        StateReadOp::Constraint(Op::Push(2)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)), // [extern_key]
        // player_pos_hash
        StateReadOp::Constraint(Op::Push(10)),
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::Constraint(Op::Access(Access::DecisionVarRange)), // [extern_key, player_pos_hash]
        StateReadOp::Constraint(Op::Push(4)),
        StateReadOp::State(State::StateReadWordRangeExtern), // [mem_address]
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let read = serde_json::to_vec(&read).unwrap();
    state_read.push(read);

    // constraint is_some(board_pos') && board_pos' == move;
    let constraint = vec![
        Op::Push(0),
        Op::Push(1),
        Op::Access(Access::StateIsSome),
        Op::Push(0),
        Op::Push(1),
        Op::Access(Access::State),
        Op::Push(1),
        Op::Access(Access::DecisionVar),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
    ];
    add_constraint(&mut constraints, constraint);

    let my_account = server.generate_account().unwrap();
    let my_key = server.get_public_key(my_account).unwrap();

    // constraint is_some(player_pos') && player_pos' == ${my_address};
    let mut constraint: Vec<_> = (0..4)
        .flat_map(|i| {
            [
                Op::Push(1 + i),
                Op::Push(1),
                Op::Access(Access::StateIsSome),
                Op::Push(1 + i),
                Op::Push(1),
                Op::Access(Access::State),
                Op::Push(my_key[i as usize]),
                Op::Pred(Pred::Eq),
                Op::Pred(Pred::And),
                Op::Pred(Pred::And),
            ]
        })
        .collect();
    // Remove first And
    constraint.remove(9);
    add_constraint(&mut constraints, constraint);

    let move_one_intent = Intent {
        slots: Slots {
            state: vec![
                StateSlot {
                    index: 0,
                    amount: 1,
                    program_index: 0,
                },
                StateSlot {
                    index: 1,
                    amount: 4,
                    program_index: 1,
                },
            ],
            decision_variables: 14,
            permits: 1,
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    };

    let move_one_intent_address = server.submit_intent(move_one_intent).unwrap();
    let pos = 2;
    let the_move = 1;
    let mut decision_variables = vec![pos, the_move];
    decision_variables.extend(&deployed_address);
    decision_variables.extend(&board_address);
    decision_variables.extend(add_to_key(player_address, 0, 4 * pos).unwrap());

    let mut game_dec_vars = board_address.to_vec();
    game_dec_vars.extend(&player_address);

    let solution = Solution {
        data: [
            SolutionData {
                intent_to_solve: SourceAddress::transient(move_one_intent_address.into()),
                decision_variables,
                sender: eoa_sender(),
            },
            SolutionData {
                intent_to_solve: SourceAddress::persistent(
                    deployed_address.into(),
                    game_intent_address.into(),
                ),
                decision_variables: game_dec_vars,
                sender: Sender::transient([0; 4], move_one_intent_address.into()),
            },
        ]
        .into_iter()
        .collect(),
        state_mutations: vec![StateMutation {
            address: deployed_address.into(),
            mutations: vec![
                Mutation::Key(KeyMutation {
                    key: add_to_key(board_address, 0, pos).unwrap(),
                    value: Some(the_move),
                }),
                Mutation::Range(RangeMutation {
                    key_range: add_to_key(player_address, 0, pos * 4).unwrap()
                        ..add_to_key(player_address, 0, pos * 4 + 4).unwrap(),
                    values: my_key.iter().map(|&k| Some(k)).collect(),
                }),
            ],
        }],
    };

    server.check(solution).unwrap();
}
