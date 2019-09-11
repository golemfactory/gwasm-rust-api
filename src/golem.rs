//! Convenience async functions for creating gWasm tasks, connecting to a
//! Golem instance, and listening for task's progress as it's computed
//! on Golem.
use super::{
    error::Error,
    task::{ComputedTask, Task},
    Net, ProgressUpdate,
};
use actix::{Actor, ActorContext, Context, Handler, Message};
use actix_wamp::RpcEndpoint;
use futures::{
    future,
    stream::{self, Stream},
    Future,
};
use golem_rpc_api::{
    comp::{AsGolemComp, TaskStatus as GolemTaskStatus},
    connect_to_app,
};
use serde_json::json;
use std::{convert::TryInto, path::Path, time::Duration};
use tokio::timer::Interval;
use tokio_ctrlc_error::AsyncCtrlc;

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
pub fn compute<P, S>(
    datadir: P,
    address: S,
    port: u16,
    task: Task,
    net: Net,
    progress_handler: impl ProgressUpdate + 'static,
    polling_interval: Option<Duration>,
) -> impl Future<Item = ComputedTask, Error = Error> + 'static
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    create_task(datadir.as_ref(), address.as_ref(), port, net, task.clone())
        .and_then(move |(endpoint, task_id)| {
            poll_task_progress(endpoint.clone(), task_id.clone(), polling_interval)
                .fold(
                    ProgressActor::new(progress_handler).start(),
                    |addr, task_status| {
                        addr.send(Update {
                            progress: task_status.progress,
                        })
                        .and_then(|_| Ok(addr))
                    },
                )
                .and_then(|addr| addr.send(Finish).map_err(Error::from))
                .ctrlc_as_error()
                .or_else(move |e: Error| match e {
                    Error::KeyboardInterrupt(e) => {
                        future::Either::A(endpoint.as_golem_comp().abort_task(task_id).then(
                            |res| match res {
                                Ok(()) => future::err(Error::KeyboardInterrupt(e)),
                                Err(e) => future::err(e.into()),
                            },
                        ))
                    }
                    e => future::Either::B(future::err(e)),
                })
        })
        .and_then(|()| task.try_into())
}

/// A convenience function for creating a gWasm [`Task`] on Golem
///
/// This function returns to necessary components to track the `Task` on Golem Network:
/// 1) an object implementing [`RpcEndpoint`] trait, 2) created `Task`'s ID as `String`.
///
/// [`Task`]: ../task/struct.Task.html
/// [`RpcEndpoint`]:
/// https://golemfactory.github.io/golem-client/latest/actix_wamp/trait.RpcEndpoint.html
pub fn create_task(
    datadir: &Path,
    address: &str,
    port: u16,
    net: Net,
    task: Task,
) -> impl Future<Item = (impl Clone + Send + RpcEndpoint, String), Error = Error> + 'static {
    connect_to_app(datadir, Some(net), Some((address, port)))
        .and_then(move |endpoint| {
            endpoint
                .as_golem_comp()
                .create_task(json!(task))
                .map(|task_id| (endpoint, task_id))
        })
        .from_err()
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
) -> impl Stream<Item = TaskStatus, Error = Error> + 'static {
    stream::unfold(TaskState::new(endpoint, task_id), |state| {
        if let Some(status) = state.task_status.status {
            match status {
                GolemTaskStatus::Finished => return None,
                GolemTaskStatus::Aborted => {
                    return Some(future::Either::A(future::err(Error::TaskAborted)))
                }
                GolemTaskStatus::Timeout => {
                    return Some(future::Either::A(future::err(Error::TaskTimedOut)))
                }
                _ => {}
            }
        }

        let mut next_state = TaskState::new(state.endpoint.clone(), state.task_id.clone());
        Some(future::Either::B(
            state
                .endpoint
                .as_golem_comp()
                .get_task(state.task_id.clone())
                .map_err(Error::from)
                .and_then(move |task_info| {
                    let task_info = task_info.ok_or(Error::EmptyTaskInfo)?;
                    next_state.task_status.status = Some(task_info.status);
                    next_state.task_status.progress =
                        task_info.progress.ok_or(Error::EmptyProgress)?;
                    Ok((next_state.task_status.clone(), next_state))
                }),
        ))
    })
    .zip(
        Interval::new_interval(polling_interval.unwrap_or_else(|| Duration::from_secs(2)))
            .from_err(),
    )
    .map(|(x, _)| x)
}

#[derive(Message)]
struct Update {
    progress: f64,
}

#[derive(Message)]
struct Finish;

struct ProgressActor<T>
where
    T: ProgressUpdate + 'static,
{
    handler: T,
}

impl<T> ProgressActor<T>
where
    T: ProgressUpdate + 'static,
{
    fn new(handler: T) -> Self {
        Self { handler }
    }
}

impl<T> Actor for ProgressActor<T>
where
    T: ProgressUpdate + 'static,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.handler.start()
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.handler.stop()
    }
}

impl<T> Handler<Update> for ProgressActor<T>
where
    T: ProgressUpdate + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: Update, _ctx: &mut Self::Context) -> Self::Result {
        self.handler.update(msg.progress);
    }
}

impl<T> Handler<Finish> for ProgressActor<T>
where
    T: ProgressUpdate + 'static,
{
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
