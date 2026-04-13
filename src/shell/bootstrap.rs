use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::shell_integration::ShellType;

pub struct ShellBootstrap {
    shell_type: ShellType,
    shell_path: PathBuf,
    env_vars: HashMap<String, String>,
    term_program: String,
    term_features: Vec<String>,
}

impl ShellBootstrap {
    pub fn new(shell_path: PathBuf) -> Self {
        let shell_type = ShellType::from_path(&shell_path).unwrap_or(ShellType::Bash);

        Self {
            shell_type,
            shell_path,
            env_vars: HashMap::new(),
            term_program: "foxix".to_string(),
            term_features: vec!["xterm-256color".to_string(), "kitty".to_string()],
        }
    }

    pub fn from_shell_name(name: &str) -> Self {
        let path = which(name).unwrap_or_else(|| PathBuf::from(format!("/bin/{}", name)));
        Self::new(path)
    }

    pub fn shell_type(&self) -> ShellType {
        self.shell_type
    }

    pub fn shell_path(&self) -> &PathBuf {
        &self.shell_path
    }

    pub fn set_env_var(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env_vars.insert(key.into(), value.into());
    }

    pub fn set_term_program(&mut self, name: impl Into<String>) {
        self.term_program = name.into();
    }

    pub fn add_term_feature(&mut self, feature: impl Into<String>) {
        self.term_features.push(feature.into());
    }

    pub fn build_env(&self) -> HashMap<String, String> {
        let mut env = env::vars().collect::<HashMap<_, _>>();

        env.insert(
            "TERM".to_string(),
            self.term_features
                .first()
                .cloned()
                .unwrap_or_else(|| "xterm-256color".to_string()),
        );
        env.insert("TERMINFO".to_string(), "/usr/share/terminfo".to_string());
        env.insert("COLORTERM".to_string(), "true".to_string());
        env.insert("TERM_PROGRAM".to_string(), self.term_program.clone());

        env.insert("FOXIX".to_string(), "1".to_string());
        env.insert(
            "FOXIX_VERSION".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );

        if let Ok(term_program_version) = env::var("TERM_PROGRAM_VERSION") {
            env.insert("FOXIX_TERM_VERSION".to_string(), term_program_version);
        }

        for (key, value) in &self.env_vars {
            env.insert(key.clone(), value.clone());
        }

        env
    }

    pub fn generate_launch_script(&self) -> String {
        let mut script = String::new();

        script.push_str("# Foxix Shell Bootstrap\n");
        script.push_str("# Generated automatically - do not edit\n\n");

        for (key, value) in self.build_env() {
            script.push_str(&format!("export {}={}\n", key, escape_shell_value(&value)));
        }

        script.push_str("\n");
        script
    }

    pub fn write_integration_files(&self, xdg_data_home: &PathBuf) -> std::io::Result<()> {
        let integration_dir = xdg_data_home.join("foxix").join("shell-integration");
        std::fs::create_dir_all(&integration_dir)?;

        let shell_name = self.shell_type.name();
        match self.shell_type {
            ShellType::Zsh => {
                let zsh_dir = integration_dir.join("zsh");
                std::fs::create_dir_all(&zsh_dir)?;

                let bootstrap_content = self.generate_zsh_bootstrap();
                std::fs::write(zsh_dir.join("foxix-integration"), bootstrap_content)?;

                let completion = self.generate_zsh_completion();
                std::fs::write(zsh_dir.join("_foxix"), completion)?;
            }
            ShellType::Fish => {
                let fish_dir = integration_dir.join("fish");
                std::fs::create_dir_all(&fish_dir)?;

                let conf_content = self.generate_fish_conf();
                std::fs::write(fish_dir.join("foxix-integration.fish"), conf_content)?;
            }
            ShellType::Bash => {
                let bash_dir = integration_dir.join("bash");
                std::fs::create_dir_all(&bash_dir)?;

                let bootstrap_content = self.generate_bash_bootstrap();
                std::fs::write(bash_dir.join("foxix-integration.bash"), bootstrap_content)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn generate_zsh_bootstrap(&self) -> String {
        let mut s = String::new();
        s.push_str("# Foxix shell integration for Zsh\n");
        s.push_str("# Add to ~/.zshrc: source ~/.local/share/foxix/shell-integration/zsh/foxix-integration\n\n");

        s.push_str("# Set terminal variables\n");
        s.push_str(&format!(
            "export TERM={}\n",
            self.term_features
                .first()
                .unwrap_or(&"xterm-256color".to_string())
        ));
        s.push_str("export COLORTERM=true\n");
        s.push_str("export FOXIX=1\n");

        s.push_str("\n# Enable bracketed paste\n");
        s.push_str("export TERM=xterm-256color\n");

        s.push_str("\n# Foxix terminfo\n");
        s.push_str("export TERMINFO=/usr/share/terminfo\n");

        s
    }

    fn generate_zsh_completion(&self) -> String {
        let mut s = String::new();
        s.push_str("#compdef _foxix foxix\n\n");
        s.push_str("_foxix() {\n");
        s.push_str("    local -a commands\n");
        s.push_str("    commands=(\n");
        s.push_str("        'launch:Launch a new Foxix instance'\n");
        s.push_str("        'remote:Remote control Foxix'\n");
        s.push_str("        'diff:Show diff of two files'\n");
        s.push_str("        'ssh:SSH with Foxix as remote terminal'\n");
        s.push_str("    )\n");
        s.push_str("    _describe 'command' commands\n");
        s.push_str("}\n\n");
        s.push_str("_foxix \"$@\"\n");
        s
    }

    fn generate_fish_conf(&self) -> String {
        let mut s = String::new();
        s.push_str("# Foxix shell integration for Fish\n");
        s.push_str("# Add to ~/.config/fish/config.fish: source ~/.local/share/foxix/shell-integration/fish/foxix-integration.fish\n\n");

        s.push_str("# Set terminal variables\n");
        s.push_str(&format!(
            "set -gx TERM {}\n",
            self.term_features
                .first()
                .unwrap_or(&"xterm-256color".to_string())
        ));
        s.push_str("set -gx COLORTERM true\n");
        s.push_str("set -gx FOXIX 1\n");

        s.push_str("\n# Enable bracketed paste\n");
        s.push_str("set -gx TERM xterm-256color\n");

        s.push_str("\n# Foxix terminfo\n");
        s.push_str("set -gx TERMINFO /usr/share/terminfo\n");

        s
    }

    fn generate_bash_bootstrap(&self) -> String {
        let mut s = String::new();
        s.push_str("# Foxix shell integration for Bash\n");
        s.push_str("# Add to ~/.bashrc: source ~/.local/share/foxix/shell-integration/bash/foxix-integration.bash\n\n");

        s.push_str("# Set terminal variables\n");
        s.push_str(&format!(
            "export TERM={}\n",
            self.term_features
                .first()
                .unwrap_or(&"xterm-256color".to_string())
        ));
        s.push_str("export COLORTERM=true\n");
        s.push_str("export FOXIX=1\n");

        s.push_str("\n# Enable bracketed paste\n");
        s.push_str("export TERM=xterm-256color\n");

        s.push_str("\n# Foxix terminfo\n");
        s.push_str("export TERMINFO=/usr/share/terminfo\n");

        s
    }

    pub fn spawn_interactive(&self) -> std::io::Result<std::process::Command> {
        let mut cmd = Command::new(&self.shell_path);
        cmd.envs(self.build_env());
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        match self.shell_type {
            ShellType::Fish => {
                cmd.arg("-i");
            }
            ShellType::Zsh => {
                cmd.arg("-i");
                cmd.arg("-l");
            }
            ShellType::Bash => {
                cmd.arg("--login");
                cmd.arg("-i");
            }
            _ => {}
        }

        Ok(cmd)
    }
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let path_str = path.to_str()?;
    for dir in path_str.split(':') {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn escape_shell_value(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    if value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('\"', "\\\""))
}
