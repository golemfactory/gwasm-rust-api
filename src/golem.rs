//! Convenience async functions for creating gWasm tasks, connecting to a
//! Golem instance, and listening for task's progress as it's computed
//! on Golem.
use super::error::{Error, Result};
use super::task::{ComputedTask, Task};
use super::{Net, ProgressUpdate};
use actix::{Actor, ActorContext, Context, Handler, Message};
use actix_wamp::RpcEndpoint;
use futures::future::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use futures::{pin_mut, select};
use golem_rpc_api::comp::{AsGolemComp, TaskStatus as GolemTaskStatus};
use golem_rpc_api::connect_to_app;
use serde_json::json;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;
use tokio::{signal, time};

/// A convenience function for running a gWasm [`Task`] on Golem
///
/// This function is essentially an async equivalent of [`gwasm_api::compute`] with
/// two exceptions: 1) it returns a future [`ComputedTask`], and 2) it optionally allows
/// to specify the polling interval for the task's updates (which by default is set to 2secs).
///
/// Note that since the function returns a future, you'll need to set up actix's event loop
/// to actually execute it, much like it's done for you in [`gwasm_api::compute`].
///
/// [`Task`]: ../task/struct.Task.html
/// [`ComputedTask`]: ../task/struct.ComputedTask.html
/// [`gwasm_api::compute`]: ../fn.compute.html
pub async fn compute<P, S>(
    datadir: P,
    address: S,
    port: u16,
    task: Task,
    net: Net,
    progress_handler: impl ProgressUpdate + 'static,
    polling_interval: Option<Duration>,
) -> Result<ComputedTask>
where
    P: Into<PathBuf>,
    S: Into<String>,
{
    let (endpoint, task_id) =
        create_task(&datadir.into(), &address.into(), port, net, task.clone()).await?;
    let poll_stream = poll_task_progress(endpoint.clone(), task_id.clone(), polling_interval);
    let progress = poll_stream
        .try_fold(
            ProgressActor::new(progress_handler).start(),
            |addr, task_status| async move {
                addr.send(Update {
                    progress: task_status.progress,
                })
                .await?;
                Ok(addr)
            },
        )
        .fuse();
    let ctrlc = signal::ctrl_c().fuse();

    pin_mut!(ctrlc, progress);

    select! {
        maybe_ctrlc = ctrlc => {
            maybe_ctrlc?;
            Err(Error::KeyboardInterrupt)
        }
        maybe_addr = progress => {
            let addr = maybe_addr?;
            addr.send(Finish).await?;
            let task: ComputedTask = task.try_into()?;
            Ok(task)
        }
    }
}

/// A convenience function for creating a gWasm [`Task`] on Golem
///
/// This function returns to necessary components to track the `Task` on Golem Network:
/// 1) an object implementing [`RpcEndpoint`] trait, 2) created `Task`'s ID as `String`.
///
/// [`Task`]: ../task/struct.Task.html
/// [`RpcEndpoint`]:
/// https://golemfactory.github.io/golem-client/latest/actix_wamp/trait.RpcEndpoint.html
pub async fn create_task(
    datadir: &Path,
    address: &str,
    port: u16,
    net: Net,
    task: Task,
) -> Result<(impl Clone + Send + RpcEndpoint, String)> {
    let endpoint = connect_to_app(datadir, Some(net), Some((address, port))).await?;
    let task_id = endpoint.as_golem_comp().create_task(json!(task)).await?;
    Ok((endpoint, task_id))
}

/// A convenience function for polling gWasm [`Task`]'s computation progress on Golem
///
/// This function returns an async [`Stream`] which can be asynchronously
/// iterated for new progress updates. Note however that this function will actively poll
/// for the updates rather than subscribe to some event publisher at a `polling_interval`
/// which if not specified by default equals 2secs.
///
/// [`Task`]: ../task/struct.Task.html
/// [`Stream`]: https://docs.rs/futures/0.1.28/futures/stream/trait.Stream.html
pub fn poll_task_progress(
    endpoint: impl Clone + Send + RpcEndpoint + 'static,
    task_id: String,
    polling_interval: Option<Duration>,
) -> impl Stream<Item = Result<TaskStatus>> {
    stream::try_unfold(TaskState::new(endpoint, task_id), |state| async move {
        if let Some(status) = state.task_status.status {
            match status {
                GolemTaskStatus::Finished => return Ok(None),
                GolemTaskStatus::Aborted => return Err(Error::TaskAborted),
                GolemTaskStatus::Timeout => return Err(Error::TaskTimedOut),
                _ => {}
            }
        }

        let mut next_state = TaskState::new(state.endpoint.clone(), state.task_id.clone());
        let task_info = state
            .endpoint
            .as_golem_comp()
            .get_task(state.task_id.clone())
            .await?;
        let task_info = task_info.ok_or(Error::EmptyTaskInfo)?;
        next_state.task_status.status = Some(task_info.status);
        next_state.task_status.progress = task_info.progress.ok_or(Error::EmptyProgress)?;
        Ok(Some((next_state.task_status.clone(), next_state)))
    })
    .zip(time::interval(
        polling_interval.unwrap_or_else(|| Duration::from_secs(2)),
    ))
    .map(|(x, _)| x)
}

struct Update {
    progress: f64,
}

impl Message for Update {
    type Result = ();
}

struct Finish;

impl Message for Finish {
    type Result = ();
}

struct ProgressActor {
    handler: Pin<Box<dyn ProgressUpdate>>,
}

impl ProgressActor {
    fn new<T: ProgressUpdate + 'static>(handler: T) -> Self {
        let handler = Box::pin(handler);
        Self { handler }
    }
}

impl Actor for ProgressActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.handler.start()
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.handler.stop()
    }
}

impl Handler<Update> for ProgressActor {
    type Result = ();

    fn handle(&mut self, msg: Update, _ctx: &mut Self::Context) -> Self::Result {
        self.handler.update(msg.progress);
    }
}

impl Handler<Finish> for ProgressActor {
    type Result = ();

    fn handle(&mut self, _msg: Finish, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}

struct TaskState<Endpoint>
where
    Endpoint: Clone + Send + RpcEndpoint + 'static,
{
    endpoint: Endpoint,
    task_id: String,
    task_status: TaskStatus,
}

impl<Endpoint> TaskState<Endpoint>
where
    Endpoint: Clone + Send + RpcEndpoint + 'static,
{
    fn new(endpoint: Endpoint, task_id: String) -> Self {
        Self {
            endpoint,
            task_id,
            task_status: TaskStatus::default(),
        }
    }
}

/// Stores current status of gWasm task
#[derive(Clone)]
pub struct TaskStatus {
    status: Option<GolemTaskStatus>,
    progress: f64,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self {
            status: None,
            progress: 0.0,
        }
    }
}
