#[cfg(any(target_os = "macos", test))]
const MAX_ANCESTORS: usize = 32;

#[cfg(any(target_os = "macos", test))]
pub(crate) fn walk(from: i32, parent: impl Fn(i32) -> Option<i32>, app: impl Fn(i32) -> bool) -> Option<i32> {
    let mut pid = from;
    for _ in 0..MAX_ANCESTORS {
        if pid <= 1 {
            return None;
        }
        if app(pid) {
            return Some(pid);
        }
        pid = parent(pid)?;
    }
    None
}

#[cfg(target_os = "macos")]
pub(crate) use imp::{bring_forward, looking, terminal};

#[cfg(not(target_os = "macos"))]
pub(crate) fn terminal() -> Option<i32> {
    None
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn looking(_terminal: Option<i32>) -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn bring_forward(_terminal: Option<i32>) {}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::c_int;

    use objc2_app_kit::{
        NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace,
    };

    use super::walk;

    fn pid_info<T>(pid: i32, flavor: c_int) -> Option<T> {
        let mut info: T = unsafe { std::mem::zeroed() };
        let size = c_int::try_from(size_of::<T>()).ok()?;
        let read = unsafe { libc::proc_pidinfo(pid, flavor, 0, (&raw mut info).cast(), size) };
        (read == size).then_some(info)
    }

    pub(super) fn parent(pid: i32) -> Option<i32> {
        let info: libc::proc_bsdshortinfo = pid_info(pid, libc::PROC_PIDT_SHORTBSDINFO)?;
        i32::try_from(info.pbsi_ppid).ok()
    }

    pub(crate) fn terminal() -> Option<i32> {
        let regular = |pid| {
            NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
                .is_some_and(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
        };
        walk(unsafe { libc::getppid() }, parent, regular)
    }

    pub(crate) fn looking(terminal: Option<i32>) -> bool {
        let front = NSWorkspace::sharedWorkspace().frontmostApplication().map(|app| app.processIdentifier());
        terminal.is_some() && front == terminal
    }

    pub(crate) fn bring_forward(terminal: Option<i32>) {
        if let Some(app) = terminal.and_then(NSRunningApplication::runningApplicationWithProcessIdentifier) {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        }
    }
}

#[cfg(test)]
mod tests;
