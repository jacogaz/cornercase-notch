use super::*;

fn lock_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cornercase-notch-test-{}-{name}.lock", std::process::id()))
}

#[test]
fn only_one_holder_at_a_time_and_the_next_takes_over() {
    let path = lock_file("holders");

    let first = try_acquire(&path).expect("open").expect("free");
    assert!(try_acquire(&path).expect("open").is_none());
    drop(first);

    assert!(try_acquire(&path).expect("open").is_some());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_lock_lives_in_the_temporary_folder() {
    assert_eq!(path().file_name().and_then(|name| name.to_str()), Some("cornercase-notch.lock"));
}
