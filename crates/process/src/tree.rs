//! Process tree management (sysinfo).

/// Kill a process and all of its child processes.
///
/// # Errors
///
/// Returns an error if the process tree cannot be enumerated or killed.
pub fn kill_tree(_pid: u32) -> crab_common::Result<()> {
    todo!()
}
