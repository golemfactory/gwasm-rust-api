#![deny(
    // missing_docs,
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
pub mod golem;
pub mod task;
pub mod timeout;

use actix::System;
use error::Error;
pub use golem_rpc_api::Net;
use std::path::Path;
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
    net: Net,
    task: Task,
    progress_handler: impl ProgressUpdate + 'static,
) -> Result<ComputedTask>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let mut system = System::new(task.name());
    system.block_on(golem::compute(
        datadir,
        address,
        port,
        task,
        net,
        progress_handler,
        None,
    ))
}

pub mod prelude {
    pub use super::task::{
        ComputedSubtask, ComputedTask, GWasmBinary, Options, Subtask, Task, TaskBuilder,
    };
    pub use super::timeout::Timeout;
    pub use super::{compute, Net, ProgressUpdate};
}
