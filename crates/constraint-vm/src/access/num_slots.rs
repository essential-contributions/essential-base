use super::*;

#[test]
fn test_num_slots() {
    let pre = std::iter::repeat(vec![42, 88]).take(22).collect::<Vec<_>>();
    let post = std::iter::repeat(vec![1, 2, 3]).take(7).collect::<Vec<_>>();
    let state = StateSlots {
        pre: &pre,
        post: &post,
    };
    let vars = std::iter::repeat(vec![-1, -2, -3, 900])
        .take(12)
        .collect::<Vec<_>>();

    let mut stack = crate::Stack::default();

    stack.push(0).unwrap();
    super::num_slots(&mut stack, &state, &vars).unwrap();
    assert_eq!(stack.pop().unwrap(), 12);

    stack.push(1).unwrap();
    super::num_slots(&mut stack, &state, &vars).unwrap();
    assert_eq!(stack.pop().unwrap(), 22);

    stack.push(2).unwrap();
    super::num_slots(&mut stack, &state, &vars).unwrap();
    assert_eq!(stack.pop().unwrap(), 7);

    stack.push(3).unwrap();
    super::num_slots(&mut stack, &state, &vars).unwrap_err();
}
