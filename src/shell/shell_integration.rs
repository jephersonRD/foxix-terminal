use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Dash,
    Sh,
}

impl ShellType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" | "fishshell" => Some(Self::Fish),
            "dash" => Some(Self::Dash),
            "sh" | "ash" => Some(Self::Sh),
            _ => None,
        }
    }

    pub fn from_path(path: &PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_lowercase();
        Self::from_str(&name)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Dash => "dash",
            Self::Sh => "sh",
        }
    }

    pub fn config_file(&self) -> &'static str {
        match self {
            Self::Bash => ".bashrc",
            Self::Zsh => ".zshrc",
            Self::Fish => "config.fish",
            Self::Dash => ".dashrc",
            Self::Sh => ".shrc",
        }
    }

    pub fn completion_dir(&self) -> &'static str {
        match self {
            Self::Bash => "/usr/share/bash-completion/completions",
            Self::Zsh => "/usr/share/zsh/site-functions",
            Self::Fish => "/usr/share/fish/completions",
            _ => "",
        }
    }
}

impl Default for ShellType {
    fn default() -> Self {
        Self::Bash
    }
}

pub struct ShellIntegration {
    shell_type: ShellType,
}

impl ShellIntegration {
    pub fn new(shell_type: ShellType) -> Self {
        Self { shell_type }
    }

    pub fn for_current_shell() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_path = PathBuf::from(shell);
        let shell_type = ShellType::from_path(&shell_path).unwrap_or(ShellType::Bash);
        Self::new(shell_type)
    }

    pub fn shell_type(&self) -> ShellType {
        self.shell_type
    }

    pub fn generate_init_script(&self) -> String {
        match self.shell_type {
            ShellType::Zsh => self.generate_zsh_init(),
            ShellType::Fish => self.generate_fish_init(),
            ShellType::Bash => self.generate_bash_init(),
            _ => String::new(),
        }
    }

    fn generate_zsh_init(&self) -> String {
        r#"# Foxix Zsh Integration
export FOXIX=1
export TERM=xterm-256color
export COLORTERM=true
"#
        .to_string()
    }

    fn generate_fish_init(&self) -> String {
        r#"# Foxix Fish Integration
set -gx FOXIX 1
set -gx TERM xterm-256color
set -gx COLORTERM true
"#
        .to_string()
    }

    fn generate_bash_init(&self) -> String {
        r#"# Foxix Bash Integration
export FOXIX=1
export TERM=xterm-256color
export COLORTERM=true
"#
        .to_string()
    }
}
