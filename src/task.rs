use super::error::Error;
use super::timeout::Timeout;
use super::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug)]
pub struct GWasmBinary {
    pub js: &'static [u8],
    pub wasm: &'static [u8],
}

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

    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = Some(name.as_ref().to_owned());
        self
    }

    pub fn bid(mut self, bid: f64) -> Self {
        self.bid = Some(bid);
        self
    }

    pub fn timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn subtask_timeout(mut self, subtask_timeout: Timeout) -> Self {
        self.subtask_timeout = Some(subtask_timeout);
        self
    }

    pub fn push_subtask_data<T: Into<Vec<u8>>>(mut self, data: T) -> Self {
        self.subtask_data.push(data.into());
        self
    }

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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bid(&self) -> f64 {
        self.bid
    }

    pub fn timeout(&self) -> &Timeout {
        &self.timeout
    }

    pub fn subtask_timeout(&self) -> &Timeout {
        &self.subtask_timeout
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
}

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

    pub fn js_name(&self) -> &str {
        &self.js_name
    }

    pub fn wasm_name(&self) -> &str {
        &self.wasm_name
    }

    pub fn input_dir_path(&self) -> &Path {
        &self.input_dir_path
    }

    pub fn output_dir_path(&self) -> &Path {
        &self.output_dir_path
    }

    pub fn subtasks(&self) -> impl Iterator<Item = (&str, &Subtask)> {
        self.subtasks
            .iter()
            .map(|(name, subtask)| (name.as_str(), subtask))
    }
}

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

    pub fn exec_args(&self) -> impl Iterator<Item = &str> {
        self.exec_args.iter().map(|s| s.as_str())
    }

    pub fn output_file_paths(&self) -> impl Iterator<Item = &Path> {
        self.output_file_paths.iter().map(|p| p.as_ref())
    }
}

#[derive(Debug)]
pub struct ComputedTask {
    pub name: String,
    pub bid: f64,
    pub timeout: Timeout,
    pub subtask_timeout: Timeout,
    pub subtasks: Vec<ComputedSubtask>,
}

#[derive(Debug)]
pub struct ComputedSubtask {
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
