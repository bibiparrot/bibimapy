use crate::{
    config::{AppConfig, AppPaths, venv_python},
    error::{AppError, AppResult},
    locale,
};
use serde::Serialize;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::Read,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const DEFAULT_NOTEBOOK: &str = include_str!("../resources/home.py");

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
struct EnvironmentStamp {
    python_version: String,
    pip_index_url: String,
    marimo_package: String,
}

impl From<&AppConfig> for EnvironmentStamp {
    fn from(config: &AppConfig) -> Self {
        Self {
            python_version: config.python_version.clone(),
            pip_index_url: config.pip_index_url.clone(),
            marimo_package: config.marimo_package.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub phase: String,
    pub detail: Option<String>,
    pub url: Option<String>,
}

impl RuntimeInfo {
    fn phase(phase: &str) -> Self {
        Self {
            phase: phase.into(),
            detail: None,
            url: None,
        }
    }
}

pub struct SharedRuntime {
    process: Mutex<Option<Child>>,
    info: Mutex<RuntimeInfo>,
    startup: Mutex<()>,
}

impl SharedRuntime {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            process: Mutex::new(None),
            info: Mutex::new(RuntimeInfo::phase("stopped")),
            startup: Mutex::new(()),
        })
    }

    pub fn status(&self) -> RuntimeInfo {
        self.info
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn update(&self, phase: &str, detail: Option<String>, url: Option<String>) {
        *self
            .info
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = RuntimeInfo {
            phase: phase.into(),
            detail,
            url,
        };
    }

    pub fn start(self: &Arc<Self>, config: &AppConfig, paths: &AppPaths) -> AppResult<RuntimeInfo> {
        let _startup = self
            .startup
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        {
            let mut process = self
                .process
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(child) = process.as_mut() {
                if child
                    .try_wait()
                    .map_err(|error| AppError::io("marimo", error))?
                    .is_none()
                {
                    return Ok(self.status());
                }
            }
            *process = None;
        }

        let result = self.start_inner(config, paths);
        if let Err(error) = &result {
            self.update("error", Some(error.to_string()), None);
        }
        result
    }

    #[allow(clippy::too_many_lines)]
    fn start_inner(&self, config: &AppConfig, paths: &AppPaths) -> AppResult<RuntimeInfo> {
        paths.ensure_directories()?;
        self.update("preparing", None, None);
        let uv = locate_uv()?;
        let environment = uv_environment(paths);
        let expected_stamp = EnvironmentStamp::from(config);
        let existing_stamp = read_environment_stamp(paths);

        self.update("python", Some(config.python_version.clone()), None);
        run_uv(
            &uv,
            ["python", "install", config.python_version.as_str()],
            paths,
            &environment,
            "installing Python",
        )?;

        let python = venv_python(paths);
        let python_changed = existing_stamp
            .as_ref()
            .is_some_and(|stamp| stamp.python_version != config.python_version);
        if !python.exists() || python_changed {
            self.update("venv", None, None);
            run_uv_os(
                &uv,
                [
                    OsString::from("venv"),
                    paths.venv.as_os_str().to_owned(),
                    OsString::from("--python"),
                    OsString::from(&config.python_version),
                    OsString::from("--clear"),
                ],
                paths,
                &environment,
                "creating the virtual environment",
            )?;
        }

        if !python_has_marimo(&python) || existing_stamp.as_ref() != Some(&expected_stamp) {
            self.update("marimo", Some(config.pip_index_url.clone()), None);
            let mut install_environment = environment.clone();
            install_environment.push((
                OsString::from("UV_DEFAULT_INDEX"),
                OsString::from(&config.pip_index_url),
            ));
            run_uv_os(
                &uv,
                [
                    OsString::from("pip"),
                    OsString::from("install"),
                    OsString::from("--python"),
                    python.as_os_str().to_owned(),
                    OsString::from(&config.marimo_package),
                ],
                paths,
                &install_environment,
                "installing marimo",
            )?;
            write_environment_stamp(paths, &expected_stamp)?;
        }

        let notebook = paths.notebooks.join("home.py");
        if !notebook.exists() {
            fs::write(&notebook, DEFAULT_NOTEBOOK)
                .map_err(|error| AppError::io(&notebook, error))?;
        }

        let port = select_port(config.marimo_port)?;
        let url = format!("http://127.0.0.1:{port}");
        self.update("server", Some(url.clone()), None);
        let log_path = paths.logs.join("marimo.log");
        let stdout = File::create(&log_path).map_err(|error| AppError::io(&log_path, error))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| AppError::io(&log_path, error))?;
        let mut child = Command::new(&python)
            .args([
                OsStr::new("-m"),
                OsStr::new("marimo"),
                OsStr::new("edit"),
                notebook.as_os_str(),
                OsStr::new("--host"),
                OsStr::new("127.0.0.1"),
                OsStr::new("--port"),
                OsStr::new(&port.to_string()),
                OsStr::new("--headless"),
                OsStr::new("--no-token"),
            ])
            .current_dir(&paths.notebooks)
            .env("BIBIMAPY_LOCALE", locale::effective_locale(config))
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| AppError::io(&python, error))?;

        let started = Instant::now();
        let timeout = Duration::from_secs(config.startup_timeout_seconds);
        loop {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| AppError::io(&python, error))?
            {
                return Err(AppError::MarimoExited(format!(
                    "exit status {status}; {}",
                    tail_log(&log_path)
                )));
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::MarimoTimeout(config.startup_timeout_seconds));
            }
            thread::sleep(Duration::from_millis(250));
        }

        *self
            .process
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(child);
        self.update("ready", None, Some(url));
        Ok(self.status())
    }

    pub fn stop(&self) {
        let mut process = self
            .process
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(mut child) = process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.update("stopped", None, None);
    }
}

fn locate_uv() -> AppResult<PathBuf> {
    if let Some(path) = env::var_os("BIBIMAPY_UV").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }

    let executable_name = if cfg!(windows) { "uv.exe" } else { "uv" };
    if let Ok(current) = env::current_exe() {
        if let Some(parent) = current.parent() {
            let sidecar = parent.join(executable_name);
            if sidecar.is_file() {
                return Ok(sidecar);
            }
        }
    }

    if let Some(path) = find_on_path(executable_name) {
        return Ok(path);
    }
    Err(AppError::UvNotFound)
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn uv_environment(paths: &AppPaths) -> Vec<(OsString, OsString)> {
    vec![
        (
            OsString::from("UV_PYTHON_INSTALL_DIR"),
            paths.python.as_os_str().to_owned(),
        ),
        (
            OsString::from("UV_CACHE_DIR"),
            paths.cache.as_os_str().to_owned(),
        ),
    ]
}

fn run_uv<'a>(
    uv: &Path,
    args: impl IntoIterator<Item = &'a str>,
    paths: &AppPaths,
    environment: &[(OsString, OsString)],
    action: &str,
) -> AppResult<()> {
    run_uv_os(
        uv,
        args.into_iter().map(OsString::from),
        paths,
        environment,
        action,
    )
}

fn run_uv_os(
    uv: &Path,
    args: impl IntoIterator<Item = OsString>,
    paths: &AppPaths,
    environment: &[(OsString, OsString)],
    action: &str,
) -> AppResult<()> {
    let output = Command::new(uv)
        .args(args)
        .current_dir(&paths.root)
        .envs(environment.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .map_err(|error| AppError::io(uv, error))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Err(AppError::Uv {
        action: action.into(),
        message: if stderr.is_empty() { stdout } else { stderr },
    })
}

fn python_has_marimo(python: &Path) -> bool {
    Command::new(python)
        .args(["-c", "import marimo"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn read_environment_stamp(paths: &AppPaths) -> Option<EnvironmentStamp> {
    fs::read_to_string(&paths.environment)
        .ok()
        .and_then(|source| toml::from_str(&source).ok())
}

fn write_environment_stamp(paths: &AppPaths, stamp: &EnvironmentStamp) -> AppResult<()> {
    let source = toml::to_string_pretty(stamp)
        .map_err(|error| AppError::InvalidConfig(error.to_string()))?;
    fs::write(&paths.environment, source).map_err(|error| AppError::io(&paths.environment, error))
}

fn select_port(preferred: u16) -> AppResult<u16> {
    (preferred..=preferred.saturating_add(100))
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or(AppError::PortUnavailable(preferred))
}

fn tail_log(path: &Path) -> String {
    let mut source = String::new();
    if File::open(path)
        .and_then(|mut file| file.read_to_string(&mut source))
        .is_err()
    {
        return "no marimo log was available".into();
    }
    let mut lines: Vec<_> = source.lines().rev().take(12).collect();
    lines.reverse();
    lines.join(" | ")
}
