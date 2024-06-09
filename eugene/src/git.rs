use log::trace;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{ContextualError, ContextualResult, InnerError};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum GitMode {
    DiffWith(String),
    Disabled,
}

impl From<Option<String>> for GitMode {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(v) => GitMode::DiffWith(v),
            None => GitMode::Disabled,
        }
    }
}

fn git_is_on_path() -> crate::Result<()> {
    Command::new("git")
        .arg("--version")
        .output()
        .map_err(|e| {
            InnerError::NoGitExecutableError
                .with_context(format!("Failed to execute `git --version`: {e}"))
        })
        .map(|_| ())
}

fn git_ref_exists<P: AsRef<Path>>(gitref: &str, cwd: P) -> crate::Result<()> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg(gitref)
        .current_dir(cwd.as_ref())
        .output()
        .map_err(|e| {
            InnerError::GitError.with_context(format!(
                "Failed to execute `git rev-parse --abbrev-ref {gitref}`: {e}"
            ))
        })
        .and_then(|o| {
            if o.status.success() {
                Ok(())
            } else {
                Err(InnerError::GitError.with_context(format!("Git ref {gitref} not found")))
            }
        })
}

/// Find the nearest directory containing the given path, useful for setting cwd for git
fn nearest_directory<P: AsRef<Path>>(path: P) -> crate::Result<PathBuf> {
    let path = path.as_ref();
    let p = Path::new(path);
    if p.is_file() {
        // p must have a parent, so we can unwrap it
        Ok(p.parent().unwrap().into())
    } else if p.is_dir() {
        Ok(p.into())
    } else if p.is_symlink() {
        // For now, symlink is not supported
        Err(InnerError::NotFound.with_context(format!(
            "{path:?} is a symlink which is unsupported by eugene::git"
        )))
    } else {
        Err(InnerError::NotFound.with_context(format!("{path:?} does not exist")))
    }
}

fn git_status<P: AsRef<Path>>(cwd: P) -> crate::Result<String> {
    let cwd = cwd.as_ref();
    Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(cwd)
        .output()
        .map_err(|e| {
            InnerError::GitError.with_context(format!(
                "Failed to execute `git status --porcelain` in {cwd:?}: {e}"
            ))
        })
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

/// Discover unstaged files in the path, which may be either a file or directory
///
/// Fails if the path does not exist, or isn't in a git repository
fn unstaged_children<P: AsRef<Path>>(path: P) -> crate::Result<Vec<String>> {
    let path = path.as_ref();
    trace!("Checking if {path:?} has unstaged");
    let cwd = nearest_directory(path)?;
    // p exists
    if path.is_file() {
        // cwd is the parent and if `git status --porcelain` inside cwd contains `?? p`
        // it is unstaged and will be the only output. We can unwrap here because `p` is a file
        let file_name = path.file_name().unwrap().to_str().ok_or_else(|| {
            InnerError::InvalidPath.with_context(format!("{path:?} contains non utf-8 characters"))
        })?;
        let status = git_status(&cwd).with_context(format!("Check if {path:?} is unstaged"))?;
        trace!("git status --porcelain in {cwd:?} is {status}");
        let look_for = format!("?? {file_name}");
        if status.lines().any(|l| l.starts_with(&look_for)) {
            let as_string = path.to_str().ok_or_else(|| {
                InnerError::InvalidPath
                    .with_context(format!("{path:?} contains non utf-8 characters"))
            })?;
            Ok(vec![as_string.to_string()])
        } else {
            Ok(vec![])
        }
    } else {
        // cwd is the directory itself. We will use it as the working dir and join all the
        // paths in the output to cwd to produce results, using only the lines that start with `??`
        let status =
            git_status(&cwd).with_context(format!("Check if {path:?} contains unstaged"))?;
        trace!("git status --porcelain in {cwd:?} is {status}");
        Ok(status
            .lines()
            .filter(|l| l.starts_with("??"))
            .map(|l| {
                let file_name = l.trim_start_matches("?? ").trim();
                cwd.join(file_name).to_str().unwrap().to_string()
            })
            .collect())
    }
}

fn git_diff_name_only(cwd: &Path, gitref: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("diff")
        .arg("--name-only")
        .arg("--relative")
        .arg(gitref)
        .current_dir(cwd);
    cmd
}

fn diff_files_since_ref<P: AsRef<Path> + Debug>(
    path: P,
    gitref: &str,
) -> crate::Result<Vec<String>> {
    let path = path.as_ref();
    let cwd = nearest_directory(path)?;
    git_ref_exists(gitref, &cwd)?;
    let mut cmd = git_diff_name_only(&cwd, gitref);
    if path.is_file() {
        // cwd is above; if `git diff --name-only` in cwd name of `path`, it changed
        let output = cmd.output().with_context(format!(
            "Failed to execute `git diff --name-only {gitref}` in {cwd:?}"
        ))?;
        let string_ouput = String::from_utf8_lossy(&output.stdout);
        trace!("git diff --name-only {gitref} in {cwd:?} is {string_ouput}");
        // We can unwrap file_name here because `p` is a file
        let file_name = path.file_name().unwrap().to_str().ok_or_else(|| {
            InnerError::InvalidPath.with_context(format!("{path:?} contains non utf-8 characters"))
        })?;
        let as_string = path.to_str().ok_or_else(|| {
            InnerError::InvalidPath.with_context(format!("{path:?} contains non utf-8 characters"))
        })?;
        if string_ouput.lines().any(|l| l == file_name) {
            Ok(vec![as_string.to_string()])
        } else {
            Ok(vec![])
        }
    } else {
        // cwd is the directory itself. We will use it as the working dir and join all the
        // paths in the output to cwd to produce results
        let output = cmd.output().with_context(format!(
            "Failed to execute `git diff --name-only {gitref}` in {cwd:?}"
        ))?;
        let string_output = String::from_utf8_lossy(&output.stdout);
        trace!("git diff --name-only {gitref} in {cwd:?} is {string_output}");
        let r: crate::Result<Vec<_>> = string_output
            .lines()
            .map(|l| {
                let file_name = l.trim();
                Ok(cwd
                    .join(file_name)
                    .to_str()
                    .ok_or_else(|| {
                        InnerError::InvalidPath
                            .with_context(format!("{path:?} contains invalid utf-8 characters"))
                    })?
                    .to_string())
            })
            .collect();
        Ok(r?)
    }
}

#[derive(Debug)]
pub struct AllowList {
    paths: Vec<String>,
}

#[derive(Debug)]
pub enum GitFilter {
    Ignore,
    OneOf(AllowList),
}

impl GitFilter {
    pub fn new<P: AsRef<Path> + Debug>(path: P, mode: GitMode) -> crate::Result<GitFilter> {
        match mode {
            GitMode::Disabled => Ok(GitFilter::Ignore),
            GitMode::DiffWith(refname) => {
                git_is_on_path()?;
                let path = path.as_ref();
                let mut diff = diff_files_since_ref(path, &refname)?;
                diff.extend(unstaged_children(path)?);
                Ok(GitFilter::OneOf(AllowList { paths: diff }))
            }
        }
    }

    pub fn empty(mode: GitMode) -> GitFilter {
        match mode {
            GitMode::Disabled => GitFilter::Ignore,
            GitMode::DiffWith(_) => GitFilter::OneOf(AllowList { paths: vec![] }),
        }
    }

    pub fn allows<S: AsRef<str>>(&self, path: S) -> bool {
        let path = path.as_ref();
        match self {
            GitFilter::Ignore => true,
            GitFilter::OneOf(allow_list) => allow_list.paths.iter().any(|p| p == path),
        }
    }

    pub fn extend(&mut self, other: GitFilter) {
        if let (GitFilter::OneOf(mine), GitFilter::OneOf(theirs)) = (self, other) {
            mine.paths.extend(theirs.paths);
        }
    }
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::*;

    struct RestoreContext {
        restore: Option<Box<dyn FnOnce()>>,
    }

    impl RestoreContext {
        fn new<F: FnOnce() + 'static>(restore: F) -> Self {
            Self {
                restore: Some(Box::new(restore)),
            }
        }
    }

    impl Drop for RestoreContext {
        fn drop(&mut self) {
            let inner = self.restore.take();
            if let Some(restore) = inner {
                restore();
            }
        }
    }

    fn set_path(new: &str) -> RestoreContext {
        let old = std::env::var("PATH").unwrap();
        std::env::set_var("PATH", new);
        RestoreContext::new(move || std::env::set_var("PATH", old))
    }

    fn configure_git(path: &Path) {
        Command::new("git")
            .arg("init")
            .arg("-b")
            .arg("main")
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("ci@example.com")
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("ci@example.com")
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_nearest_dir() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join("foo");
        std::fs::write(&fp, "").unwrap();
        assert_eq!(nearest_directory(fp).unwrap(), tmp.path());
        assert_eq!(nearest_directory(tmp.path()).unwrap(), tmp.path());
        let subdir = tmp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        assert_eq!(&nearest_directory(&subdir).unwrap(), &subdir);
        let notexists = tmp.path().join("notexists");
        assert!(nearest_directory(notexists).is_err());
    }

    #[test]
    fn test_is_git_in_path() {
        assert!(git_is_on_path().is_ok());
        let _tmp = set_path("");
        assert!(git_is_on_path().is_err());
    }

    #[test]
    fn test_unstaged() {
        let tmp = TempDir::new().unwrap();
        Command::new("git")
            .arg("init")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(unstaged_children(tmp.path().to_str().unwrap())
            .unwrap()
            .is_empty());
        assert!(unstaged_children(tmp.path().join("foo").to_str().unwrap()).is_err());
        let fp = tmp.path().join("foo");
        std::fs::write(&fp, "hei").unwrap();
        assert_eq!(
            unstaged_children(fp.to_str().unwrap()).unwrap(),
            vec![fp.to_str().unwrap()]
        );
    }

    #[test]
    fn test_gitref_exists() {
        let tmp = TempDir::new().unwrap();
        configure_git(tmp.path());
        assert!(git_ref_exists("main", tmp.path()).is_err());
        let fp = tmp.path().join("foo");
        std::fs::write(fp, "hei").unwrap();
        let o = Command::new("git")
            .arg("add")
            .arg("foo")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        eprintln!("{o:?}");
        let o = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        eprintln!("{o:?}");
        assert!(git_ref_exists("main", tmp.path()).is_ok());
        assert!(git_ref_exists("nonono", tmp.path()).is_err());
    }

    #[test]
    fn test_diff() {
        let tmp = TempDir::new().unwrap();
        configure_git(tmp.path());
        let fp = tmp.path().join("foo");
        std::fs::write(&fp, "hei").unwrap();
        Command::new("git")
            .arg("add")
            .arg("foo")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(diff_files_since_ref(&fp, "main").unwrap().is_empty(),);
        Command::new("git")
            .arg("checkout")
            .arg("-b")
            .arg("newbranch")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        let fp2 = tmp.path().join("bar");
        std::fs::write(&fp2, "hei").unwrap();
        Command::new("git")
            .arg("add")
            .arg("bar")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("new file")
            .current_dir(tmp.path())
            .output()
            .unwrap();
        // The new file is contained in the diff with main
        assert_eq!(
            diff_files_since_ref(&fp2, "main").unwrap(),
            vec![fp2.to_str().unwrap()]
        );
        assert_eq!(
            diff_files_since_ref(tmp.path(), "main").unwrap(),
            vec![fp2.to_str().unwrap()]
        );

        // Change fp
    }
}
