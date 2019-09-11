//! Convenience types for creating and managing gWasm tasks
use super::{error::Error, timeout::Timeout, Result};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Wrapper type for easy passing of gWasm binary
#[derive(Debug)]
pub struct GWasmBinary {
    /// Contents of JavaScript file generated by Emscripten
    pub js: &'static [u8],
    /// Contents of Wasm file generated by Emscripten
    pub wasm: &'static [u8],
}

/// gWasm task builder
///
/// Note that when [`build`] method is executed, the `TaskBuilder` will
/// be consumed and will generate a [`Task`] and a corresponding dir
/// and file structure in the provided `workspace` [`Path`]. For more
/// details about the dir structure, see [gWasm docs].
///
/// # Example:
/// ```
/// use gwasm_api::task::{GWasmBinary, TaskBuilder};
/// use std::path::Path;
/// use tempfile::tempdir;
///
/// let binary = GWasmBinary {
///     js: &[],
///     wasm: &[],
/// };
/// let workspace = tempdir().unwrap();
/// let task = TaskBuilder::new(&workspace, binary).build();
/// assert!(task.is_ok());
/// assert!(task.unwrap().options().subtasks().next().is_none());
/// ```
///
/// [`build`]: struct.TaskBuilder.html#method.build
/// [`Task`]: ../task/struct.Task.html
/// [`Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
/// [gWasm docs]: https://docs.golem.network/#/Products/Brass-Beta/gWASM?id=inputoutput
#[derive(Debug)]
pub struct TaskBuilder {
    binary: GWasmBinary,
    name: Option<String>,
    bid: Option<f64>,
    timeout: Option<Timeout>,
    subtask_timeout: Option<Timeout>,
    input_dir_path: PathBuf,
    output_dir_path: PathBuf,
    subtask_data: Vec<Vec<u8>>,
}

impl TaskBuilder {
    /// Creates new `TaskBuilder` from workspace `Path` and `GWasmBinary`
    pub fn new<P: AsRef<Path>>(workspace: P, binary: GWasmBinary) -> Self {
        Self {
            binary,
            name: None,
            bid: None,
            timeout: None,
            subtask_timeout: None,
            input_dir_path: workspace.as_ref().join("in"),
            output_dir_path: workspace.as_ref().join("out"),
            subtask_data: Vec::new(),
        }
    }

    /// Sets task's name
    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = Some(name.as_ref().to_owned());
        self
    }

    /// Sets task's bid value
    pub fn bid(mut self, bid: f64) -> Self {
        self.bid = Some(bid);
        self
    }

    /// Sets task's [`Timeout`](../timeout/struct.Timeout.html) value
    pub fn timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets subtasks' [`Timeout`](../timeout/struct.Timeout.html) value
    pub fn subtask_timeout(mut self, subtask_timeout: Timeout) -> Self {
        self.subtask_timeout = Some(subtask_timeout);
        self
    }

    /// Pushes subtask data into the buffer
    ///
    /// Note that each pushed chunk of `data` is equivalent to one
    /// subtask that will be executed on Golem Network.
    pub fn push_subtask_data<T: Into<Vec<u8>>>(mut self, data: T) -> Self {
        self.subtask_data.push(data.into());
        self
    }

    /// Consumes this builder and creates a `Task`
    ///
    /// Note that when this method is executed, a corresponding dir
    /// and file structure in the provided `workspace` [`Path`]. For more
    /// details about the dir structure, see [gWasm docs].
    ///
    /// [`Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
    /// [gWasm docs]: https://docs.golem.network/#/Products/Brass-Beta/gWASM?id=inputoutput
    pub fn build(mut self) -> Result<Task> {
        let name = self.name.take().unwrap_or("unknown".to_owned());
        let bid = self.bid.unwrap_or(1.0);
        let timeout = self.timeout.unwrap_or(
            Timeout::from_str("00:10:00")
                .expect("could correctly parse default task timeout value"),
        );
        let subtask_timeout = self.subtask_timeout.unwrap_or(
            Timeout::from_str("00:10:00")
                .expect("could correctly parse default subtask timeout value"),
        );
        let js_name = format!("{}.js", name);
        let wasm_name = format!("{}.wasm", name);
        let mut options = Options::new(
            js_name,
            wasm_name,
            self.input_dir_path.clone(),
            self.output_dir_path.clone(),
        );

        // create input dir
        fs::create_dir(&options.input_dir_path)?;

        // save JS file
        let js_filename = options.input_dir_path.join(&options.js_name);
        fs::write(&js_filename, self.binary.js)?;

        // save WASM file
        let wasm_filename = options.input_dir_path.join(&options.wasm_name);
        fs::write(&wasm_filename, self.binary.wasm)?;

        // create output dir
        fs::create_dir(&options.output_dir_path)?;

        // subtasks
        for (i, chunk) in self.subtask_data.into_iter().enumerate() {
            let name = format!("subtask_{}", i);

            // create input subtask dir
            let input_dir_path = options.input_dir_path.join(&name);
            fs::create_dir(&input_dir_path)?;

            // create output subtask dir
            let output_dir_path = options.output_dir_path.join(&name);
            fs::create_dir(&output_dir_path)?;

            // save input data file
            let input_name = "in.txt";
            let input_filename = input_dir_path.join(&input_name);
            fs::write(&input_filename, &chunk)?;

            let mut subtask = Subtask::new();
            subtask.exec_args.push(input_name.into());

            let output_name = "in.wav";
            subtask.exec_args.push(output_name.into());
            subtask.output_file_paths.push(output_name.into());

            options.subtasks.insert(name, subtask);
        }

        Ok(Task::new(name, bid, timeout, subtask_timeout, options))
    }
}

/// Struct representing gWasm task
///
/// This type serves two purposes: 1) it can be serialized to JSON manifest
/// required by Golem (see [gWasm Task JSON]), and 2) it tracks the dirs and files
/// created on disk which contain the actual subtasks' data and params.
///
/// Note that `Task` can only be created using the [`TaskBuilder`].
///
/// # Example:
/// ```
/// use gwasm_api::task::{GWasmBinary, TaskBuilder};
/// use std::path::Path;
/// use serde_json::json;
/// use tempfile::tempdir;
///
/// let binary = GWasmBinary {
///     js: &[],
///     wasm: &[],
/// };
/// let workspace = tempdir().unwrap();
/// let task = TaskBuilder::new(&workspace, binary).build().unwrap();
/// let json_manifest = json!(task);
/// ```
///
/// [gWasm Task JSON]: https://docs.golem.network/#/Products/Brass-Beta/gWASM?id=task-json
/// [`TaskBuilder`]: ../task/struct.TaskBuilder.html
#[derive(Debug, Serialize, Clone)]
pub struct Task {
    #[serde(rename = "type")]
    task_type: String,
    name: String,
    bid: f64,
    timeout: Timeout,
    subtask_timeout: Timeout,
    options: Options,
}

impl Task {
    fn new<S: Into<String>>(
        name: S,
        bid: f64,
        timeout: Timeout,
        subtask_timeout: Timeout,
        options: Options,
    ) -> Self {
        Self {
            task_type: "wasm".into(),
            name: name.into(),
            bid,
            timeout,
            subtask_timeout,
            options,
        }
    }

    /// Task's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Task's bid value
    pub fn bid(&self) -> f64 {
        self.bid
    }

    /// Task's [`Timeout`](../timeout/struct.Timeout.html) value
    pub fn timeout(&self) -> &Timeout {
        &self.timeout
    }

    /// Subtasks' [`Timeout`](../timeout/struct.Timeout.html) value
    pub fn subtask_timeout(&self) -> &Timeout {
        &self.subtask_timeout
    }

    /// [`Options`](../task/struct.Options.html) substructure
    pub fn options(&self) -> &Options {
        &self.options
    }
}

/// Struct representing gWasm task's options substructure
///
/// Stores information such as the name of JavaScript file, or the
/// name of Wasm binary. This struct should only ever be used in conjunction
/// with [`Task`] structure, and thus, as such, it is impossible to be created
/// on its own.
///
/// [`Task`]: ../task/struct.Task.html
#[derive(Debug, Serialize, Clone)]
pub struct Options {
    js_name: String,
    wasm_name: String,
    #[serde(rename = "input_dir")]
    input_dir_path: PathBuf,
    #[serde(rename = "output_dir")]
    output_dir_path: PathBuf,
    subtasks: BTreeMap<String, Subtask>,
}

impl Options {
    fn new<S: Into<String>, P: Into<PathBuf>>(
        js_name: S,
        wasm_name: S,
        input_dir_path: P,
        output_dir_path: P,
    ) -> Self {
        Self {
            js_name: js_name.into(),
            wasm_name: wasm_name.into(),
            input_dir_path: input_dir_path.into(),
            output_dir_path: output_dir_path.into(),
            subtasks: BTreeMap::new(),
        }
    }

    /// Name of the JavaScript file
    pub fn js_name(&self) -> &str {
        &self.js_name
    }

    /// Name of the Wasm binary
    pub fn wasm_name(&self) -> &str {
        &self.wasm_name
    }

    /// Path to the task's input dir
    pub fn input_dir_path(&self) -> &Path {
        &self.input_dir_path
    }

    /// Path to the task's output dir
    pub fn output_dir_path(&self) -> &Path {
        &self.output_dir_path
    }

    /// Returns an [`Iterator`] over created [`Subtask`]'s
    ///
    /// [`Subtask`]: ../task/struct.Subtask.html
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    pub fn subtasks(&self) -> impl Iterator<Item = (&str, &Subtask)> {
        self.subtasks
            .iter()
            .map(|(name, subtask)| (name.as_str(), subtask))
    }
}

/// Struct representing gWasm task's subtask substructure
///
/// Stores information such as the execution arguments for the Wasm binary,
/// and output file paths for the computed results.
#[derive(Debug, Serialize, Clone)]
pub struct Subtask {
    exec_args: Vec<String>,
    output_file_paths: Vec<PathBuf>,
}

impl Subtask {
    fn new() -> Self {
        Self {
            exec_args: Vec::new(),
            output_file_paths: Vec::new(),
        }
    }

    /// Returns an [`Iterator`] over the execution arguments of this subtask
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    pub fn exec_args(&self) -> impl Iterator<Item = &str> {
        self.exec_args.iter().map(|s| s.as_str())
    }

    /// Returns an [`Iterator`] over the output file paths of this subtask
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    pub fn output_file_paths(&self) -> impl Iterator<Item = &Path> {
        self.output_file_paths.iter().map(|p| p.as_ref())
    }
}

/// Struct representing computed gWasm task
///
/// This struct in addition to storing some information about the
/// computed task such as its name, or timeouts, it first and foremost
/// contains a [`Vec`] of [`ComputedSubtask`]s.
///
/// # Example:
/// ```
/// use gwasm_api::task::{GWasmBinary, TaskBuilder, ComputedTask};
/// use std::path::Path;
/// use tempfile::tempdir;
/// use std::convert::TryInto;
///
/// let binary = GWasmBinary {
///     js: &[],
///     wasm: &[],
/// };
/// let workspace = tempdir().unwrap();
/// let task = TaskBuilder::new(&workspace, binary).build().unwrap();
/// let computed_task: Result<ComputedTask, _> = task.try_into();
///
/// assert!(computed_task.is_ok());
/// assert!(computed_task.unwrap().subtasks.is_empty());
/// ```
///
/// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
/// [`ComputedSubtask`]: ../task/struct.ComputedSubtask.html
#[derive(Debug)]
pub struct ComputedTask {
    /// Task's name
    pub name: String,
    /// Used task bid value
    pub bid: f64,
    /// Used task [`Timeout`] value
    ///
    /// [`Timeout`]: ../timeout/struct.Timeout.html
    pub timeout: Timeout,
    /// Used subtask [`Timeout`] value
    ///
    /// [`Timeout`]: ../timeout/struct.Timeout.html
    pub subtask_timeout: Timeout,
    /// [`Vec`] of [`ComputedSubtask`]s, ordered by subtask data insertion
    /// using [`TaskBuilder::push_subtask_data`]
    ///
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    /// [`ComputedSubtask`]: ../task/struct.ComputedSubtask.html
    /// [`TaskBuilder::push_subtask_data`]:
    /// ../task/struct.TaskBuilder.html#method.push_subtask_data
    pub subtasks: Vec<ComputedSubtask>,
}

/// Struct representing computed subtask
///
/// It contains, for each [output file path], an instance of
/// [`BufReader`] which can be used to read the computed data from file
/// to a container.
///
/// [output file path]: ../task/struct.Subtask.html#method.output_file_paths
/// [`BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
#[derive(Debug)]
pub struct ComputedSubtask {
    /// [`BTreeMap`] of results for the `ComputedSubtask` where
    /// key is each path matching the output of [`Subtask::output_file_paths`],
    /// and the value is [`BufReader`] pointing to a file with computation
    /// results
    ///
    /// [`BTreeMap`]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
    /// [`Subtask::output_file_paths`]: ../task/struct.Subtask.html#method.output_file_paths
    /// [`BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
    pub data: BTreeMap<PathBuf, BufReader<File>>,
}

impl TryFrom<Task> for ComputedTask {
    type Error = Error;

    fn try_from(task: Task) -> Result<Self> {
        let name = task.name;
        let bid = task.bid;
        let timeout = task.timeout;
        let subtask_timeout = task.subtask_timeout;
        let mut computed_subtasks = Vec::new();

        for (s_name, subtask) in task.options.subtasks() {
            let output_dir = task.options.output_dir_path().join(s_name);
            let mut computed_subtask = ComputedSubtask {
                data: BTreeMap::new(),
            };

            for out_path in subtask.output_file_paths() {
                let f = File::open(output_dir.join(out_path))?;
                let reader = BufReader::new(f);
                computed_subtask.data.insert(out_path.into(), reader);
            }

            computed_subtasks.push(computed_subtask);
        }

        Ok(Self {
            name,
            bid,
            timeout,
            subtask_timeout,
            subtasks: computed_subtasks,
        })
    }
}
