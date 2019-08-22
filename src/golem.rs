use super::error::Error;
use super::task::Task;
use super::ProgressUpdate;
use actix::{Actor, ActorContext, Context, Handler, Message};
use actix_wamp::RpcEndpoint;
use futures::stream::{self, Stream};
use futures::{future, Future};
use golem_rpc_api::comp::AsGolemComp;
use golem_rpc_api::{connect_to_app, Net};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;
use tokio::timer::Interval;
use tokio_ctrlc_error::AsyncCtrlc;

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

pub(crate) fn compute(
    datadir: PathBuf,
    address: String,
    port: u16,
    task: Task,
    progress_handler: impl ProgressUpdate + 'static,
    polling_interval: Option<Duration>,
) -> impl Future<Item = (), Error = Error> + 'static {
    let addr = ProgressActor::new(progress_handler).start();

    create_task(datadir, address, port, task).and_then(move |(endpoint, task_id)| {
        listen_task_progress(endpoint.clone(), task_id.clone(), polling_interval)
            .fold(addr, |addr, progress| {
                addr.send(Update { progress }).and_then(|_| Ok(addr))
            })
            .and_then(|addr| addr.send(Finish).map_err(Error::from))
            .ctrlc_as_error()
            .or_else(move |e: Error| match e {
                Error::KeyboardInterrupt(e) => {
                    future::Either::A(endpoint.as_golem_comp().abort_task(task_id).then(|res| {
                        match res {
                            Ok(()) => future::err(Error::KeyboardInterrupt(e)),
                            Err(e) => future::err(e.into()),
                        }
                    }))
                }
                e => future::Either::B(future::err(e)),
            })
    })
}

fn create_task(
    datadir: PathBuf,
    address: String,
    port: u16,
    task: Task,
) -> impl Future<Item = (impl Clone + Send + RpcEndpoint, String), Error = Error> + 'static {
    connect_to_app(
        datadir.as_ref(),
        Some(Net::TestNet),
        Some((address.as_ref(), port)),
    )
    .and_then(move |endpoint| {
        endpoint
            .as_golem_comp()
            .create_task(json!(task))
            .map(|task_id| (endpoint, task_id))
    })
    .from_err()
}

fn listen_task_progress(
    endpoint: impl Clone + Send + RpcEndpoint + 'static,
    task_id: String,
    polling_interval: Option<Duration>,
) -> impl Stream<Item = f64, Error = Error> + 'static {
    stream::unfold(TaskState::new(endpoint, task_id), |state| {
        if state.progress < 1.0 {
            let mut next_state = TaskState::new(state.endpoint.clone(), state.task_id.clone());
            Some(
                state
                    .endpoint
                    .as_golem_comp()
                    .get_task(state.task_id.clone())
                    .and_then(move |task_info| {
                        next_state.progress = task_info.unwrap().progress.unwrap();
                        Ok((next_state.progress, next_state))
                    })
                    .from_err(),
            )
        } else {
            None
        }
    })
    .zip(
        Interval::new_interval(polling_interval.unwrap_or_else(|| Duration::from_secs(2)))
            .from_err(),
    )
    .map(|(x, _)| x)
}

struct TaskState<Endpoint>
where
    Endpoint: Clone + Send + RpcEndpoint + 'static,
{
    endpoint: Endpoint,
    task_id: String,
    progress: f64,
}

impl<Endpoint> TaskState<Endpoint>
where
    Endpoint: Clone + Send + RpcEndpoint + 'static,
{
    fn new(endpoint: Endpoint, task_id: String) -> Self {
        Self {
            endpoint,
            task_id,
            progress: 0.0,
        }
    }
}
