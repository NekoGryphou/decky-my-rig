use super::acquire;

#[test]
fn a_second_copy_is_refused_until_the_first_ends() {
    let name = format!("Local\\MyRig.SingleInstanceTest.{}", std::process::id());
    let first = acquire(&name).unwrap();
    assert!(first.is_some());
    assert!(acquire(&name).unwrap().is_none());
    drop(first);
    assert!(acquire(&name).unwrap().is_some());
}
