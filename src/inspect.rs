use std::fmt;
use std::path::PathBuf;

use sysinfo::{Pid, Process, Users};

/// Runtime (or kind of program) a process belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Java,
    Node,
    Deno,
    Bun,
    Python,
    DotNet,
    Ruby,
    Php,
    Perl,
    Shell,
    Docker,
    Wsl,
    Native,
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Runtime::Java => "Java",
            Runtime::Node => "Node.js",
            Runtime::Deno => "Deno",
            Runtime::Bun => "Bun",
            Runtime::Python => "Python",
            Runtime::DotNet => ".NET",
            Runtime::Ruby => "Ruby",
            Runtime::Php => "PHP",
            Runtime::Perl => "Perl",
            Runtime::Shell => "Shell",
            Runtime::Docker => "Docker",
            Runtime::Wsl => "WSL",
            Runtime::Native => "Native program",
        };
        f.write_str(label)
    }
}

/// What a runtime is actually running (script, jar, module, main class...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPoint {
    /// Short label describing the kind of entry point ("Script", "Jar", ...).
    pub kind: &'static str,
    /// The entry point itself (path, module or class name).
    pub value: String,
    /// Package the entry point comes from, if it lives in `node_modules`.
    pub package: Option<String>,
}

/// Everything worth showing about a process before killing it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub runtime: Runtime,
    pub entry: Option<EntryPoint>,
    pub command: String,
    pub exe: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub user: Option<String>,
    pub memory_bytes: u64,
    pub uptime_secs: u64,
    /// Names of all descendants that will be killed along with this process.
    pub children: Vec<String>,
}

impl ProcessInfo {
    /// Build the description of `process`, given the names of its descendants.
    pub fn from_process(pid: Pid, process: &Process, users: &Users, children: Vec<String>) -> Self {
        let name = process.name().to_string_lossy().to_string();
        let args: Vec<String> = process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        let runtime = detect_runtime(&name);
        // The first element of the command line is the executable itself.
        let entry = detect_entry(runtime, args.get(1..).unwrap_or_default());

        Self {
            pid: pid.as_u32(),
            name,
            runtime,
            entry,
            command: args.join(" "),
            exe: process.exe().map(PathBuf::from),
            cwd: process.cwd().map(PathBuf::from),
            user: process
                .user_id()
                .and_then(|uid| users.get_user_by_id(uid))
                .map(|user| user.name().to_string()),
            memory_bytes: process.memory(),
            uptime_secs: process.run_time(),
            children,
        }
    }
}

/// Normalize an executable name: lowercase, without directory nor `.exe`.
fn normalize_exe_name(name: &str) -> String {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let lower = file.to_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

/// Guess the runtime from the executable name.
pub fn detect_runtime(process_name: &str) -> Runtime {
    let name = normalize_exe_name(process_name);

    match name.as_str() {
        "java" | "javaw" => Runtime::Java,
        "node" | "nodejs" => Runtime::Node,
        "deno" => Runtime::Deno,
        "bun" => Runtime::Bun,
        "py" | "pyw" => Runtime::Python,
        "dotnet" => Runtime::DotNet,
        "ruby" => Runtime::Ruby,
        "perl" => Runtime::Perl,
        "cmd" | "powershell" | "pwsh" | "bash" | "sh" | "zsh" | "fish" | "dash" => Runtime::Shell,
        "wslrelay" | "wslhost" | "wsl" => Runtime::Wsl,
        "docker-proxy" | "dockerd" | "docker" | "vpnkit" => Runtime::Docker,
        n if n.starts_with("com.docker.") => Runtime::Docker,
        n if n.starts_with("python") => Runtime::Python,
        n if n.starts_with("php") => Runtime::Php,
        _ => Runtime::Native,
    }
}

/// Options that consume the following argument, per runtime.
fn options_with_value(runtime: Runtime) -> &'static [&'static str] {
    match runtime {
        Runtime::Java => &[
            "-cp",
            "-classpath",
            "--class-path",
            "-p",
            "--module-path",
            "--add-modules",
            "--add-opens",
            "--add-exports",
        ],
        Runtime::Node => &["-r", "--require", "--import", "--loader", "-C"],
        Runtime::Python => &["-W", "-X", "--check-hash-based-pycs"],
        Runtime::Ruby => &["-I", "-r"],
        _ => &[],
    }
}

/// Find the first positional argument, skipping options (and their values).
fn first_positional(runtime: Runtime, args: &[String]) -> Option<String> {
    let with_value = options_with_value(runtime);
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if with_value.contains(&arg.as_str()) {
            iter.next();
        } else if !arg.starts_with('-') {
            return Some(arg.clone());
        }
    }
    None
}

/// Return the argument following `flag`, if any.
fn value_after(args: &[String], flags: &[&str]) -> Option<String> {
    args.iter()
        .position(|arg| flags.contains(&arg.as_str()))
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Extract the package name from a path inside `node_modules`.
fn node_package(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let rest = normalized.rsplit_once("node_modules/")?.1;
    let mut parts = rest.split('/');
    let first = parts.next().filter(|p| !p.is_empty())?;

    if first.starts_with('@') {
        parts.next().map(|second| format!("{first}/{second}"))
    } else {
        Some(first.to_string())
    }
}

/// Figure out what the runtime is executing from its arguments.
pub fn detect_entry(runtime: Runtime, args: &[String]) -> Option<EntryPoint> {
    let entry = |kind, value: String| {
        let package = node_package(&value);
        Some(EntryPoint {
            kind,
            value,
            package,
        })
    };

    match runtime {
        Runtime::Java => {
            if let Some(jar) = value_after(args, &["-jar"]) {
                return entry("Jar", jar);
            }
            if let Some(module) = value_after(args, &["-m", "--module"]) {
                return entry("Module", module);
            }
            first_positional(runtime, args).and_then(|class| entry("Main class", class))
        }
        Runtime::Python => {
            if let Some(module) = value_after(args, &["-m"]) {
                return entry("Module", module);
            }
            first_positional(runtime, args).and_then(|script| entry("Script", script))
        }
        Runtime::Deno | Runtime::Bun => {
            let skip = usize::from(args.first().is_some_and(|a| a == "run"));
            first_positional(runtime, &args[skip..]).and_then(|script| entry("Script", script))
        }
        Runtime::DotNet => first_positional(runtime, args).and_then(|dll| entry("Assembly", dll)),
        Runtime::Node | Runtime::Ruby | Runtime::Php | Runtime::Perl => {
            first_positional(runtime, args).and_then(|script| entry("Script", script))
        }
        Runtime::Shell | Runtime::Docker | Runtime::Wsl | Runtime::Native => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_detect_runtime_known_names() {
        assert_eq!(detect_runtime("java.exe"), Runtime::Java);
        assert_eq!(detect_runtime("javaw"), Runtime::Java);
        assert_eq!(detect_runtime("node"), Runtime::Node);
        assert_eq!(detect_runtime("Node.EXE"), Runtime::Node);
        assert_eq!(detect_runtime("python3.12"), Runtime::Python);
        assert_eq!(detect_runtime("py.exe"), Runtime::Python);
        assert_eq!(detect_runtime("dotnet"), Runtime::DotNet);
        assert_eq!(detect_runtime("deno"), Runtime::Deno);
        assert_eq!(detect_runtime("bun"), Runtime::Bun);
        assert_eq!(detect_runtime("ruby"), Runtime::Ruby);
        assert_eq!(detect_runtime("php-cgi.exe"), Runtime::Php);
        assert_eq!(detect_runtime("perl"), Runtime::Perl);
        assert_eq!(detect_runtime("pwsh.exe"), Runtime::Shell);
        assert_eq!(detect_runtime("wslrelay.exe"), Runtime::Wsl);
        assert_eq!(detect_runtime("com.docker.backend.exe"), Runtime::Docker);
        assert_eq!(detect_runtime("docker-proxy"), Runtime::Docker);
    }

    #[test]
    fn test_detect_runtime_with_path() {
        assert_eq!(detect_runtime("/usr/bin/node"), Runtime::Node);
        assert_eq!(detect_runtime(r"C:\jdk\bin\java.exe"), Runtime::Java);
    }

    #[test]
    fn test_detect_runtime_unknown_is_native() {
        assert_eq!(detect_runtime("nginx"), Runtime::Native);
        assert_eq!(detect_runtime(""), Runtime::Native);
    }

    #[test]
    fn test_runtime_display() {
        assert_eq!(Runtime::Node.to_string(), "Node.js");
        assert_eq!(Runtime::DotNet.to_string(), ".NET");
        assert_eq!(Runtime::Native.to_string(), "Native program");
        assert_eq!(Runtime::Java.to_string(), "Java");
    }

    #[test]
    fn test_detect_entry_java_jar() {
        let entry = detect_entry(
            Runtime::Java,
            &args(&["-Xmx1g", "-jar", "app.jar", "--port"]),
        );
        assert_eq!(
            entry.map(|e| (e.kind, e.value)),
            Some(("Jar", "app.jar".into()))
        );
    }

    #[test]
    fn test_detect_entry_java_main_class_skips_classpath() {
        let entry = detect_entry(
            Runtime::Java,
            &args(&["-cp", "lib/*", "-Dfoo=bar", "com.example.Main", "arg"]),
        );
        assert_eq!(
            entry.map(|e| (e.kind, e.value)),
            Some(("Main class", "com.example.Main".into()))
        );
    }

    #[test]
    fn test_detect_entry_java_module() {
        let entry = detect_entry(Runtime::Java, &args(&["-m", "my.mod/my.Main"]));
        assert_eq!(entry.map(|e| e.kind), Some("Module"));
    }

    #[test]
    fn test_detect_entry_node_script_skips_require() {
        let entry = detect_entry(
            Runtime::Node,
            &args(&["--inspect", "-r", "dotenv/config", "server.js"]),
        );
        let entry = entry.expect("should detect script");
        assert_eq!(entry.kind, "Script");
        assert_eq!(entry.value, "server.js");
        assert_eq!(entry.package, None);
    }

    #[test]
    fn test_detect_entry_node_package() {
        let entry = detect_entry(
            Runtime::Node,
            &args(&[r"C:\app\node_modules\vite\bin\vite.js", "dev"]),
        );
        assert_eq!(entry.and_then(|e| e.package), Some("vite".into()));
    }

    #[test]
    fn test_detect_entry_node_scoped_package() {
        let entry = detect_entry(
            Runtime::Node,
            &args(&["/app/node_modules/@angular/cli/bin/ng.js", "serve"]),
        );
        assert_eq!(entry.and_then(|e| e.package), Some("@angular/cli".into()));
    }

    #[test]
    fn test_detect_entry_python_module_and_script() {
        let module = detect_entry(Runtime::Python, &args(&["-m", "http.server", "8000"]));
        assert_eq!(
            module.map(|e| (e.kind, e.value)),
            Some(("Module", "http.server".into()))
        );

        let script = detect_entry(Runtime::Python, &args(&["-u", "main.py"]));
        assert_eq!(
            script.map(|e| (e.kind, e.value)),
            Some(("Script", "main.py".into()))
        );
    }

    #[test]
    fn test_detect_entry_deno_and_bun_run() {
        let deno = detect_entry(Runtime::Deno, &args(&["run", "--allow-net", "main.ts"]));
        assert_eq!(deno.map(|e| e.value), Some("main.ts".into()));

        let bun = detect_entry(Runtime::Bun, &args(&["index.ts"]));
        assert_eq!(bun.map(|e| e.value), Some("index.ts".into()));
    }

    #[test]
    fn test_detect_entry_dotnet() {
        let entry = detect_entry(Runtime::DotNet, &args(&["MyApi.dll"]));
        assert_eq!(
            entry.map(|e| (e.kind, e.value)),
            Some(("Assembly", "MyApi.dll".into()))
        );
    }

    #[test]
    fn test_detect_entry_none_cases() {
        assert_eq!(detect_entry(Runtime::Native, &args(&["foo"])), None);
        assert_eq!(detect_entry(Runtime::Shell, &args(&["/c", "x"])), None);
        assert_eq!(detect_entry(Runtime::Node, &[]), None);
        assert_eq!(detect_entry(Runtime::Node, &args(&["--inspect"])), None);
        assert_eq!(detect_entry(Runtime::Java, &args(&["-jar"])), None);
        assert_eq!(detect_entry(Runtime::Deno, &args(&["run"])), None);
    }

    #[test]
    fn test_node_package_outside_node_modules() {
        assert_eq!(node_package("/app/server.js"), None);
        assert_eq!(node_package("/app/node_modules/"), None);
        assert_eq!(node_package("/app/node_modules/@scope"), None);
    }

    #[test]
    fn test_process_info_from_current_process() {
        let system = crate::process::create_system();
        let pid = sysinfo::get_current_pid().expect("should get current pid");
        let process = system.process(pid).expect("current process exists");
        let users = Users::new();

        let info = ProcessInfo::from_process(pid, process, &users, vec!["child".into()]);
        assert_eq!(info.pid, pid.as_u32());
        assert!(!info.name.is_empty());
        assert_eq!(info.children, vec!["child".to_string()]);
    }
}
