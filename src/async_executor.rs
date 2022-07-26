//! Provide pluggable asynchronous executors
#![warn(missing_docs)]
#![warn(unsafe_code)]
use std::future::Future;

#[cfg(feature = "async_tokio")]
use tokio::task::JoinHandle;

/// AsyncExecutor provides a simplified interface to abstract running a set of
/// tasks on different runtimes.
pub trait AsyncExecutor {
    /// Simplified function to run the current tasks added to the
    /// executor.  This function expects you to call spawn beforehand
    /// to add tasks to the task set, then call run.
    /// Blocks until it completes.
    fn run(self);

    /// Run the executor on the given runtime.  Blocks until it is
    /// complete.  Passing in async blocks with multiple await points
    /// may fail in this executor.
    fn block_on<T: 'static>(self, future: impl Future<Output = T> + 'static + std::marker::Send);

    /// Spawn a new task on this executor.
    fn spawn<T: 'static + std::marker::Send>(
        &self,
        future: impl Future<Output = T> + 'static + std::marker::Send,
    ) -> JoinHandle<T>;
}

/// A generic asynchronous runner.
/// These can be implemented by different runtimes, such as Tokio or async-std
#[cfg(feature = "async_tokio")]
pub struct AsyncRunner {
    /// The runtime that executes tasks
    pub rt: tokio::runtime::Runtime,
    /// The task group to add tasks to
    pub local_set: tokio::task::LocalSet,
}

#[cfg(feature = "async_tokio")]
impl Default for AsyncRunner {
    fn default() -> AsyncRunner {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build();
        let rt = match rt {
            Ok(result) => result,
            Err(e) => {
                panic!("Couldn't create a Tokio runtime: {}", e);
            }
        };

        // Create a Set to run our tasks in
        let local_set = tokio::task::LocalSet::new();

        AsyncRunner { rt, local_set }
    }
}

#[cfg(feature = "async_tokio")]
impl AsyncExecutor for AsyncRunner {
    fn run(self) {
        // Start the local task set
        self.rt.block_on(self.local_set);
    }

    fn block_on<T: 'static>(self, future: impl Future<Output = T> + 'static + std::marker::Send) {
        // Spawn the given task on the local task set
        self.local_set.spawn_local(future);

        // Start the local task set
        self.rt.block_on(self.local_set)
    }

    fn spawn<T: 'static>(
        &self,
        future: impl Future<Output = T> + 'static + std::marker::Send,
    ) -> JoinHandle<T> {
        self.local_set.spawn_local(future)
    }
}
