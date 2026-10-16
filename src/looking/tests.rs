use super::*;

#[test]
fn the_terminal_is_the_first_app_up_the_tree() {
    assert_eq!(walk(10, |pid| Some(pid - 1), |pid| pid == 6), Some(6));
}

#[test]
fn a_parent_that_cannot_be_read_ends_the_search() {
    assert_eq!(walk(10, |pid| (pid > 8).then_some(pid - 1), |pid| pid == 6), None);
}

#[test]
fn no_app_up_to_launchd_means_none() {
    assert_eq!(walk(10, |pid| Some(pid - 1), |_| false), None);
}

#[cfg(target_os = "macos")]
mod parent {
    use super::super::imp::parent;

    #[test]
    fn is_read_from_a_process_of_another_user() {
        assert_eq!(parent(1), Some(0));
    }

    #[test]
    fn of_this_process_is_its_parent() {
        let ours = Some(unsafe { libc::getppid() });

        assert_eq!(parent(i32::try_from(std::process::id()).expect("a pid")), ours);
    }
}
