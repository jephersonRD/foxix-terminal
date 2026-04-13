use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use crate::terminal::AnsiParser;

pub trait Kitten: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn run(&self, ctx: &KittenContext, args: &[String]) -> KittenResult;
    fn supports_remote_control(&self) -> bool;
}

pub struct KittenContext {
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub rows: usize,
    pub cols: usize,
    pub parser: Option<*mut AnsiParser>,
    pub input: Option<Box<dyn Fn(&str) + Send>>,
    pub output: Option<Box<dyn Fn(&[u8]) + Send>>,
    pub window_id: Option<usize>,
    pub tab_id: Option<usize>,
}

impl KittenContext {
    pub fn new(cwd: PathBuf, rows: usize, cols: usize) -> Self {
        Self {
            cwd,
            env: std::env::vars().collect(),
            rows,
            cols,
            parser: None,
            input: None,
            output: None,
            window_id: None,
            tab_id: None,
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_parser(mut self, parser: *mut AnsiParser) -> Self {
        self.parser = Some(parser);
        self
    }

    pub fn with_io(
        mut self,
        input: impl Fn(&str) + Send + 'static,
        output: impl Fn(&[u8]) + Send + 'static,
    ) -> Self {
        self.input = Some(Box::new(input));
        self.output = Some(Box::new(output));
        self
    }

    pub fn with_window_id(mut self, id: usize) -> Self {
        self.window_id = Some(id);
        self
    }

    pub fn with_tab_id(mut self, id: usize) -> Self {
        self.tab_id = Some(id);
        self
    }
}

#[derive(Debug, Clone)]
pub enum KittenResult {
    Success,
    Output(String),
    Error(String),
    Partial(String),
    Exit(i32),
}

impl KittenResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Output(_) | Self::Partial(_))
    }

    pub fn message(&self) -> String {
        match self {
            Self::Success => "OK".to_string(),
            Self::Output(s) => s.clone(),
            Self::Error(s) => s.clone(),
            Self::Partial(s) => s.clone(),
            Self::Exit(code) => format!("Exited with code {}", code),
        }
    }
}

pub struct KittenRegistry {
    kittens: HashMap<&'static str, Box<dyn Kitten>>,
    custom_paths: Vec<PathBuf>,
    enabled: bool,
}

impl KittenRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            kittens: HashMap::new(),
            custom_paths: vec![],
            enabled: true,
        };

        registry.register_builtin_kittens();
        registry
    }

    fn register_builtin_kittens(&mut self) {}

    pub fn register(&mut self, kitten: Box<dyn Kitten>) {
        let name = kitten.name();
        self.kittens.insert(name, kitten);
    }

    pub fn register_builtin<K: Kitten + 'static>(&mut self, kitten: K) {
        self.kittens.insert(kitten.name(), Box::new(kitten));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Kitten> {
        self.kittens.get(name).map(|k| k.as_ref())
    }

    pub fn list(&self) -> Vec<(&'static str, &'static str)> {
        self.kittens
            .iter()
            .map(|(name, kitten)| (*name, kitten.description()))
            .collect()
    }

    pub fn add_custom_path(&mut self, path: PathBuf) {
        self.custom_paths.push(path);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn run_kitten(
        &self,
        name: &str,
        ctx: &KittenContext,
        args: &[String],
    ) -> Option<KittenResult> {
        if !self.enabled {
            return Some(KittenResult::Error("Kittens disabled".to_string()));
        }

        if let Some(kitten) = self.get(name) {
            Some(kitten.run(ctx, args))
        } else {
            self.run_external_kitten(name, ctx, args)
        }
    }

    fn run_external_kitten(
        &self,
        name: &str,
        ctx: &KittenContext,
        args: &[String],
    ) -> Option<KittenResult> {
        for path in &self.custom_paths {
            let kitten_path = path.join(name);
            if kitten_path.exists() {
                return self.execute_kitten_script(&kitten_path, ctx, args);
            }
        }

        let user_kittens = dirs::data_local_dir()
            .map(|d| d.join("foxix").join("kittens"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/foxix/kittens"));

        let kitten_path = user_kittens.join(name);
        if kitten_path.exists() {
            return self.execute_kitten_script(&kitten_path, ctx, args);
        }

        None
    }

    fn execute_kitten_script(
        &self,
        path: &PathBuf,
        ctx: &KittenContext,
        args: &[String],
    ) -> Option<KittenResult> {
        let mut cmd = std::process::Command::new(path);
        cmd.envs(&ctx.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().ok()?;

        if let Some(ref input_fn) = ctx.input {
            if let Some(ref mut stdin) = child.stdin {
                let input = format!(
                    "{}\n{}\n{}\n{}\n",
                    ctx.cwd.display(),
                    ctx.rows,
                    ctx.cols,
                    args.join(" ")
                );
                input_fn(&input);
                let _ = stdin.write_all(input.as_bytes());
            }
        }

        let output = child.wait().ok()?;

        if output.success() {
            Some(KittenResult::Success)
        } else {
            Some(KittenResult::Exit(output.code().unwrap_or(-1)))
        }
    }
}

impl Default for KittenRegistry {
    fn default() -> Self {
        Self::new()
    }
}
