use std::io;
use tokio::task::JoinHandle;

/// Spawn a blocking tokio task with name if tokio_unstable flag is set
pub fn spawn_blocking_task_with_name_if_supported<Function, Output>(
    _sometimes_used_name: &str,
    function: Function,
) -> io::Result<JoinHandle<Output>>
where
    Function: FnOnce() -> Output + Send + 'static,
    Output: Send + 'static,
{
    #[cfg(all(tokio_unstable, feature = "console"))]
    {
        tokio::task::Builder::new()
            .name(_sometimes_used_name)
            .spawn_blocking(function)
    }

    #[cfg(not(all(tokio_unstable, feature = "console")))]
    {
        Ok(tokio::task::spawn_blocking(function))
    }
}
