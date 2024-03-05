use std::io;
use tokio::task::JoinHandle;

/// Spawn a blocking tokio task with name if tokio_unstable flag is set
pub fn spawn_blocking_task_with_name_if_supported<Function, Output>(
    name: &str,
    function: Function,
) -> io::Result<JoinHandle<Output>>
where
    Function: FnOnce() -> Output + Send + 'static,
    Output: Send + 'static,
{
    #[cfg(tokio_unstable)]
    {
        tokio::task::Builder::new()
            .name(name)
            .spawn_blocking(function)
    }

    #[cfg(not(tokio_unstable))]
    tokio::task::spawn_blocking(function)
}
