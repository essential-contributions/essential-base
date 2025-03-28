use super::*;
use crate::vm::asm;
use asm::short::*;
use essential_types::convert::word_from_bytes;

fn p(ops: &[asm::Op]) -> Program {
    Program(asm::to_bytes(ops.iter().copied()).collect())
}

#[test]
fn test_check_program_for_post_state_read() {
    // Sanity
    assert!(!check_program_for_post_state_read(&p(&[POP, POP, POP])));

    // Regular key range
    assert!(!check_program_for_post_state_read(&p(&[KRNG])));

    // Post key range
    assert!(check_program_for_post_state_read(&p(&[PKRNG])));

    // Post key range extern
    assert!(check_program_for_post_state_read(&p(&[PKREX])));

    // Push contains key opcode
    let key: u8 = PKRNG.to_opcode().into();
    let bytes = [key, 0, 0, 0, 0, 0, 0, 0];
    let word = word_from_bytes(bytes);

    assert!(!check_program_for_post_state_read(&p(&[PUSH(word)])));

    // Push doesn't prevent detection
    assert!(check_program_for_post_state_read(&p(&[PUSH(word), PKRNG])));

    // Empty
    assert!(!check_program_for_post_state_read(&p(&[])));
}
