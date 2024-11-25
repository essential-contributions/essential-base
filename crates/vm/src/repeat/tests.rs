use super::*;

#[test]
fn test_repeat_from() {
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

#[test]
fn test_repeat_to() {
    let mut repeat = Repeat::new();
    repeat.repeat_to(0, 2).unwrap();
    repeat.repeat_to(1, 2).unwrap();

    assert_eq!(repeat.counter().unwrap(), 0);
    assert_eq!(repeat.repeat().unwrap(), Some(1));
    assert_eq!(repeat.counter().unwrap(), 1);
    assert_eq!(repeat.repeat().unwrap(), None);
    assert_eq!(repeat.repeat().unwrap(), Some(0));
    assert_eq!(repeat.counter().unwrap(), 1);
    assert_eq!(repeat.repeat().unwrap(), None);
    repeat.repeat().unwrap_err();
}
