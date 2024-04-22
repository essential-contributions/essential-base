use super::*;

#[test]
fn test_repeat() {
    let mut repeat = Repeat::new();
    repeat.repeat_from(0, 2).unwrap();
    repeat.repeat_from(1, 2).unwrap();

    assert_eq!(repeat.counter().unwrap(), 2);
    assert_eq!(repeat.repeat().unwrap(), Some(1));
    assert_eq!(repeat.counter().unwrap(), 1);
    assert_eq!(repeat.repeat().unwrap(), None);
    assert_eq!(repeat.repeat().unwrap(), Some(0));
    assert_eq!(repeat.counter().unwrap(), 1);
    assert_eq!(repeat.repeat().unwrap(), None);
    repeat.repeat().unwrap_err();
}
