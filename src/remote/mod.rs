use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, TempDir};

use crate::config::QemuConfig;
use crate::error::{VexError, VexResult};
use crate::utils::io::prompt_user_default_no;

const DEFAULT_TAG: &str = "latest";
const DEFAULT_BRANCH: &str = "main";
const REMOTE_URL_ENV: &str = "VEX_REMOTE_URL";
const REMOTE_BRANCH_ENV: &str = "VEX_REMOTE_BRANCH";
const GIT_USER_NAME_ENV: &str = "VEX_REMOTE_GIT_NAME";
const GIT_USER_EMAIL_ENV: &str = "VEX_REMOTE_GIT_EMAIL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSpec {
    pub id: String,
    pub name: String,
    pub tag: Option<String>,
}

impl RemoteSpec {
    pub fn parse(input: &str) -> VexResult<Self> {
        let (id, remainder) = input
            .split_once('/')
            .ok_or_else(|| VexError::RemoteSpecInvalid {
                input: input.to_string(),
                reason: "must be in the form <id/name>[:tag]".to_string(),
            })?;
        let (name, tag) = match remainder.split_once(':') {
            Some((name, tag)) => (name, Some(tag)),
            None => (remainder, None),
        };

        validate_segment("id", id)?;
        validate_segment("name", name)?;

        if let Some(tag) = tag {
            validate_segment("tag", tag)?;
        }

        Ok(Self {
            id: id.to_string(),
            name: name.to_string(),
            tag: tag.map(ToOwned::to_owned),
        })
    }

    pub fn resolved_tag(&self) -> &str {
        self.tag.as_deref().unwrap_or(DEFAULT_TAG)
    }

    fn tag_path(&self) -> PathBuf {
        registry_root()
            .join(&self.id)
            .join(&self.name)
            .join(format!("{}.json", self.resolved_tag()))
    }

    fn latest_path(&self) -> PathBuf {
        registry_root()
            .join(&self.id)
            .join(&self.name)
            .join(format!("{}.json", DEFAULT_TAG))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedConfig {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub tag: String,
    pub config: QemuConfig,
}

impl PublishedConfig {
    fn new(spec: &RemoteSpec, tag: impl Into<String>, config: QemuConfig) -> Self {
        Self {
            schema_version: 1,
            id: spec.id.clone(),
            name: spec.name.clone(),
            tag: tag.into(),
            config,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Cancelled,
    NoChanges,
    Pushed,
}

pub fn clone_remote_repo() -> VexResult<(TempDir, PathBuf)> {
    let remote_url = remote_url()?;
    let branch = remote_branch();

    let temp_dir = Builder::new()
        .prefix("vex-remote-")
        .tempdir()
        .map_err(|e| VexError::IoError {
            path: PathBuf::from("/tmp"),
            operation: "create temporary directory".to_string(),
            source: e,
        })?;
    let worktree = temp_dir.path().join("repo");

    run_git(
        temp_dir.path(),
        &["clone", &remote_url, &path_to_string(&worktree)?],
    )?;
    prepare_worktree(&worktree, &branch)?;

    Ok((temp_dir, worktree))
}

pub fn load_published_config(worktree: &Path, spec: &RemoteSpec) -> VexResult<PublishedConfig> {
    let remote_path = worktree.join(spec.tag_path());
    if !remote_path.exists() {
        return Err(VexError::RemoteConfigNotFound {
            id: spec.id.clone(),
            name: spec.name.clone(),
            tag: spec.resolved_tag().to_string(),
        });
    }

    let content = fs::read_to_string(&remote_path).map_err(|e| VexError::IoError {
        path: remote_path.clone(),
        operation: "read remote configuration file".to_string(),
        source: e,
    })?;

    if let Ok(published) = serde_json::from_str::<PublishedConfig>(&content) {
        return Ok(published);
    }

    let config: QemuConfig =
        serde_json::from_str(&content).map_err(|e| VexError::ConfigParseFailed { source: e })?;
    Ok(PublishedConfig::new(
        spec,
        spec.resolved_tag().to_string(),
        config,
    ))
}

pub fn publish_config(
    spec: &RemoteSpec,
    config: &QemuConfig,
    force: bool,
) -> VexResult<PublishOutcome> {
    let (_temp_dir, worktree) = clone_remote_repo()?;
    let published_tag = spec.resolved_tag().to_string();
    let target_path = worktree.join(spec.tag_path());

    if target_path.exists() && !force {
        println!(
            "Remote configuration '{} / {}:{}' already exists, overwrite? [y/N]",
            spec.id, spec.name, published_tag
        );
        if !prompt_user_default_no()? {
            println!("Push cancelled");
            return Ok(PublishOutcome::Cancelled);
        }
    }

    let published = PublishedConfig::new(spec, published_tag.clone(), config.clone());
    write_published_config(&target_path, &published)?;

    if published_tag != DEFAULT_TAG {
        let latest_path = worktree.join(spec.latest_path());
        write_published_config(&latest_path, &published)?;
    }

    run_git(&worktree, &["add", "."])?;
    if git_status_is_clean(&worktree)? {
        return Ok(PublishOutcome::NoChanges);
    }

    configure_commit_identity(&worktree)?;
    let message = format!("Publish {} / {}:{}", spec.id, spec.name, published_tag);
    run_git(&worktree, &["commit", "-m", &message])?;

    let branch = remote_branch();
    run_git(&worktree, &["push", "origin", &branch])?;

    Ok(PublishOutcome::Pushed)
}

fn validate_segment(label: &str, value: &str) -> VexResult<()> {
    if value.is_empty() {
        return Err(VexError::RemoteSpecInvalid {
            input: value.to_string(),
            reason: format!("{} cannot be empty", label),
        });
    }

    if value.contains('\0') {
        return Err(VexError::RemoteSpecInvalid {
            input: value.to_string(),
            reason: format!("{} cannot contain null bytes", label),
        });
    }

    if value.len() > 255 {
        return Err(VexError::RemoteSpecInvalid {
            input: value.to_string(),
            reason: format!("{} exceeds 255 characters", label),
        });
    }

    if matches!(value, "." | "..") {
        return Err(VexError::RemoteSpecInvalid {
            input: value.to_string(),
            reason: format!("{} cannot be '.' or '..'", label),
        });
    }

    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(VexError::RemoteSpecInvalid {
            input: value.to_string(),
            reason: format!(
                "{} '{}' contains unsupported characters. Use letters, numbers, '.', '-' or '_' only",
                label, value
            ),
        });
    }

    Ok(())
}

fn registry_root() -> PathBuf {
    PathBuf::from("configs")
}

fn remote_url() -> VexResult<String> {
    match env::var(REMOTE_URL_ENV) {
        Ok(value) if !value.trim().is_empty() => normalize_remote_url(
            value.trim(),
            &env::current_dir().map_err(|e| VexError::IoError {
                path: PathBuf::from("."),
                operation: "get current directory".to_string(),
                source: e,
            })?,
        ),
        _ => Err(VexError::RemoteNotConfigured {
            env_var: REMOTE_URL_ENV.to_string(),
        }),
    }
}

fn normalize_remote_url(value: &str, cwd: &Path) -> VexResult<String> {
    if is_explicit_git_url(value) {
        return Ok(value.to_string());
    }

    let remote_path = PathBuf::from(value);
    if remote_path.is_absolute() {
        return Ok(value.to_string());
    }

    path_to_string(&cwd.join(remote_path))
}

fn remote_branch() -> String {
    match env::var(REMOTE_BRANCH_ENV) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => DEFAULT_BRANCH.to_string(),
    }
}

fn prepare_worktree(worktree: &Path, branch: &str) -> VexResult<()> {
    let origin_branch = format!("origin/{}", branch);
    let origin_ref = format!("refs/remotes/origin/{}", branch);

    if git_succeeds(worktree, &["show-ref", "--verify", &origin_ref]) {
        run_git(worktree, &["checkout", "-B", branch, &origin_branch])?;
        return Ok(());
    }

    if git_succeeds(worktree, &["fetch", "origin", branch]) {
        run_git(worktree, &["checkout", "-B", branch, "FETCH_HEAD"])?;
        return Ok(());
    }

    let has_commits = git_succeeds(worktree, &["rev-parse", "--verify", "HEAD"]);
    if has_commits {
        return Err(VexError::GitCommandFailed {
            args: format!("fetch origin {}", branch),
            stderr: format!(
                "Remote branch '{}' does not exist or could not be fetched",
                branch
            ),
            stdout: String::new(),
            exit_code: None,
        });
    }

    if !git_succeeds(worktree, &["checkout", "-B", branch]) {
        run_git(worktree, &["checkout", "--orphan", branch])?;
    }

    Ok(())
}

fn configure_commit_identity(worktree: &Path) -> VexResult<()> {
    let user_name = env::var(GIT_USER_NAME_ENV).unwrap_or_else(|_| "Vex CLI".to_string());
    let user_email =
        env::var(GIT_USER_EMAIL_ENV).unwrap_or_else(|_| "vex@example.invalid".to_string());

    run_git(worktree, &["config", "user.name", &user_name])?;
    run_git(worktree, &["config", "user.email", &user_email])?;

    Ok(())
}

fn write_published_config(path: &Path, published: &PublishedConfig) -> VexResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| VexError::IoError {
            path: parent.to_path_buf(),
            operation: "create remote configuration directory".to_string(),
            source: e,
        })?;
    }

    let content = serde_json::to_string_pretty(published)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(path, content).map_err(|e| VexError::IoError {
        path: path.to_path_buf(),
        operation: "write remote configuration".to_string(),
        source: e,
    })?;

    Ok(())
}

fn git_status_is_clean(worktree: &Path) -> VexResult<bool> {
    let output = git_output(worktree, &["status", "--short"])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn run_git(worktree: &Path, args: &[&str]) -> VexResult<()> {
    let output = git_output(worktree, args)?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(VexError::GitCommandFailed {
        args: args.join(" "),
        stderr,
        stdout,
        exit_code: output.status.code(),
    })
}

fn git_succeeds(worktree: &Path, args: &[&str]) -> bool {
    git_output(worktree, args)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_output(worktree: &Path, args: &[&str]) -> VexResult<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(worktree)
        .output()
        .map_err(|e| VexError::IoError {
            path: PathBuf::from("git"),
            operation: format!("execute git {}", args.join(" ")),
            source: e,
        })
}

fn path_to_string(path: &Path) -> VexResult<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| VexError::ValidationError {
            field: None,
            reason: "path contains invalid Unicode".to_string(),
        })
}

fn is_explicit_git_url(value: &str) -> bool {
    value.contains("://") || is_scp_style_git_url(value)
}

fn is_scp_style_git_url(value: &str) -> bool {
    let Some((prefix, suffix)) = value.split_once(':') else {
        return false;
    };

    if prefix.is_empty() || suffix.is_empty() {
        return false;
    }

    if prefix.contains('/') || prefix.contains('\\') {
        return false;
    }

    if prefix.len() == 1 && prefix.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{RemoteSpec, normalize_remote_url};

    #[test]
    fn parse_remote_reference_with_tag() {
        let spec = RemoteSpec::parse("team/demo:v1").unwrap();
        assert_eq!(spec.id, "team");
        assert_eq!(spec.name, "demo");
        assert_eq!(spec.tag.as_deref(), Some("v1"));
        assert_eq!(spec.resolved_tag(), "v1");
    }

    #[test]
    fn parse_remote_reference_without_tag_uses_latest() {
        let spec = RemoteSpec::parse("team/demo").unwrap();
        assert_eq!(spec.id, "team");
        assert_eq!(spec.name, "demo");
        assert!(spec.tag.is_none());
        assert_eq!(spec.resolved_tag(), "latest");
    }

    #[test]
    fn parse_remote_reference_rejects_dot_segments() {
        let err = RemoteSpec::parse("./demo:v1").unwrap_err().to_string();
        assert!(err.contains("cannot be '.' or '..'"));

        let err = RemoteSpec::parse("team/..:v1").unwrap_err().to_string();
        assert!(err.contains("cannot be '.' or '..'"));
    }

    #[test]
    fn normalize_remote_url_resolves_relative_paths_against_cwd() {
        let cwd = Path::new("/workspace/project");
        let url = normalize_remote_url("../remote.git", cwd).unwrap();
        assert_eq!(PathBuf::from(url), cwd.join("../remote.git"));
    }

    #[test]
    fn normalize_remote_url_preserves_explicit_git_urls() {
        let cwd = Path::new("/workspace/project");
        let url = normalize_remote_url("https://github.com/example/repo.git", cwd).unwrap();
        assert_eq!(url, "https://github.com/example/repo.git");

        let url = normalize_remote_url("git@github.com:example/repo.git", cwd).unwrap();
        assert_eq!(url, "git@github.com:example/repo.git");

        let url = normalize_remote_url("alice@git.example.com:team/repo.git", cwd).unwrap();
        assert_eq!(url, "alice@git.example.com:team/repo.git");

        let url = normalize_remote_url("git.example.com:team/repo.git", cwd).unwrap();
        assert_eq!(url, "git.example.com:team/repo.git");
    }
}
