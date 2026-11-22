use super::utils::generate_with;
use super::*;

#[test]
fn os_randomness_has_valid_shape_and_independent_outputs() {
    let first = generate().unwrap();
    let second = generate().unwrap();
    assert_eq!(first.0.len(), 8);
    assert!(first.0.bytes().all(|b| b.is_ascii_digit()));
    assert_ne!(first.1, second.1);
    assert_ne!(first.2, second.2);
    assert_eq!(first.2.get_version_num(), 4);
}

#[test]
fn entropy_failures_and_rejected_samples_are_bounded() {
    assert!(generate_with(|_| Err("unavailable")).is_err());
    let mut calls = 0;
    assert!(
        generate_with(|bytes| {
            calls += 1;
            bytes.fill(255);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 18);
    let (code, _, _) = generate_with(|bytes| {
        bytes.fill(0);
        Ok(())
    })
    .unwrap();
    assert_eq!(code, "00000000");
}
