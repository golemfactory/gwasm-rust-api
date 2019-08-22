#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

pub mod error;
mod golem;
pub mod task;
pub mod timeout;

use error::Error;
use futures::future::Future;
use std::convert::TryInto;
use std::path::PathBuf;
use task::{ComputedTask, Task};

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub trait ProgressUpdate {
    fn update(&mut self, progress: f64);
    fn start(&mut self) {}
    fn stop(&mut self) {}
}

pub fn compute<P, S>(
    datadir: P,
    address: S,
    port: u16,
    task: Task,
    progress_handler: impl ProgressUpdate + 'static,
) -> impl Future<Item = ComputedTask, Error = Error>
where
    P: Into<PathBuf>,
    S: Into<String>,
{
    golem::compute(
        datadir.into(),
        address.into(),
        port,
        task.clone(),
        progress_handler,
        None,
    )
    .and_then(|()| task.try_into())
}

pub mod prelude {
    pub use super::task::{
        ComputedSubtask, ComputedTask, GWasmBinary, Options, Subtask, Task, TaskBuilder,
    };
    pub use super::timeout::Timeout;
    pub use super::{compute, ProgressUpdate};
}
