//! # gwasm-api - gWasm API for Rust apps
//! [gWasm](https://docs.golem.network/#/Products/Brass-Beta/gWASM) is Golem's new
//! meta use-case which allows Golem's developers/users to deploy their Wasm apps
//! on Golem Network. This API providers convenience structures and functions for
//! creating a gWasm task and connecting with Golem Network all from native Rust code.
//!
//! ## Example
//!
//! ```rust,no_run
//! use gwasm_api::prelude::*;
//! use anyhow::Result;
//! use std::path::PathBuf;
//!
//! struct ProgressTracker;
//!
//! impl ProgressUpdate for ProgressTracker {
//!     fn update(&self, progress: f64) {
//!         println!("Current progress = {}", progress);
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     let binary = GWasmBinary {
//!         js: &[0u8; 100],   // JavaScript file generated by Emscripten
//!         wasm: &[0u8; 100], // Wasm binary generated by Emscripten
//!     };
//!     let task = TaskBuilder::new("workspace", binary)
//!         .push_subtask_data(vec![0u8; 100])
//!         .build()?;
//!     let computed_task = compute(
//!         PathBuf::from("datadir"),
//!         "127.0.0.1".to_string(),
//!         61000,
//!         Net::TestNet,
//!         task,
//!         ProgressTracker,
//!     )?;
//!
//!     for subtask in computed_task.subtasks {
//!         for (_, reader) in subtask.data {
//!             assert!(!reader.buffer().is_empty());
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## More examples
//! * [g-flite](https://github.com/golemfactory/g-flite) is a CLI which uses `gwasm-api`
//!   internally
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
pub mod golem;
pub mod task;
pub mod timeout;

use actix::System;
use error::Result;
pub use golem_rpc_api::Net;
use std::path::PathBuf;
use task::{ComputedTask, Task};

/// Trait specifying the required interface for an object tracking the computation's
/// progress
///
/// Note that progress is tracked via active polling thus it might be prudent to store
/// the value of current progress in the struct implementing the trait and update it
/// only when the new reported progress value has actually risen
/// (see [Example: ProgressBar](#example-progressbar)).
///
/// # Example: simple tracker
/// ```
/// use gwasm_api::ProgressUpdate;
///
/// struct SimpleTracker;
///
/// impl ProgressUpdate for SimpleTracker {
///     fn update(&self, progress: f64) {
///         println!("Current progress = {}", progress);
///     }
/// }
/// ```
///
/// # Example: ProgressBar
/// ```
/// use std::cell::Cell;
/// use gwasm_api::ProgressUpdate;
/// use indicatif::ProgressBar;
///
/// struct ProgressBarTracker {
///     bar: ProgressBar,
///     progress: Cell<f64>,
/// }
///
/// impl ProgressBarTracker {
///     fn new(num_subtasks: u64) -> Self {
///         Self {
///             bar: ProgressBar::new(num_subtasks),
///             progress: Cell::new(0.0),
///         }
///     }
/// }
///
/// impl ProgressUpdate for ProgressBarTracker {
///     fn update(&self, progress: f64) {
///         if progress > self.progress.get() {
///             self.progress.set(progress);
///             self.bar.inc(1);
///         }
///     }
///
///     fn start(&self) {
///         self.bar.inc(0);
///     }
///
///     fn stop(&self) {
///         self.bar.finish_and_clear()
///     }
/// }
/// ```
pub trait ProgressUpdate {
    /// Called when progress value was polled from Golem
    fn update(&self, progress: f64);
    /// Called when progress updates started
    fn start(&self) {}
    /// Called when progress updates finished
    fn stop(&self) {}
}

/// A convenience function for running a gWasm [`Task`] on Golem
///
/// The function uses actix's `System` to spawn an event loop in the current thread,
/// and blocks until either a gWasm [`Task`] is computed, or it registers a Ctrl-C event,
/// or there was an [`Error`].
///
/// [`Task`]: task/struct.Task.html
/// [`Error`]: error/enum.Error.html
pub fn compute<P, S>(
    datadir: P,
    address: S,
    port: u16,
    net: Net,
    task: Task,
    progress_handler: impl ProgressUpdate + 'static,
) -> Result<ComputedTask>
where
    P: Into<PathBuf> + 'static,
    S: Into<String> + 'static,
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
    //! The `gwasm-api` prelude
    //!
    //! The purpose of this module is to alleviate imports of common structures and functions
    //! by adding a glob import to the top of the `gwasm-api` heavy modules:
    //!
    //! ```
    //! # #![allow(unused_imports)]
    //! use gwasm_api::prelude::*;
    //! ```
    pub use super::error::{Error, Result};
    pub use super::task::{
        ComputedSubtask, ComputedTask, GWasmBinary, Options, Subtask, Task, TaskBuilder,
    };
    pub use super::timeout::Timeout;
    pub use super::{compute, Net, ProgressUpdate};
}
