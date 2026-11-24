//! Fikra slash commands: GitHub, Kaggle and Colab workflows, account balance,
//! plans and docs. Prompt-style commands hand the agent a precise task; the
//! others run locally without calling a model.

use std::path::Path;
use std::process::Command;
use std::process::Stdio;

pub(crate) const PLANS_URL: &str = "https://fikraapi.co.ke/plans?ref=fikra-code";
pub(crate) const DOCS_URL: &str = "https://fikraapi.co.ke/code/docs?ref=fikra-code";

/// The signed-in Fikra API key: `FIKRA_API_KEY` / `OPENAI_API_KEY`, else auth.json.
pub(crate) fn api_key() -> Option<String> {
    if let Some(key) = codex_login::read_openai_api_key_from_env() {
        return Some(key);
    }
    let home = codex_utils_home_dir::find_codex_home().ok()?;
    let raw = std::fs::read_to_string(home.as_path().join("auth.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("OPENAI_API_KEY")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}

fn run_quiet(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `(owner, repo)` of the `origin` remote when it points at GitHub.
pub(crate) fn github_remote(cwd: &Path) -> Option<(String, String)> {
    let url = run_quiet(cwd, "git", &["remote", "get-url", "origin"])?;
    parse_github_remote(&url)
}

pub(crate) fn parse_github_remote(url: &str) -> Option<(String, String)> {
    let rest = url
        .trim()
        .strip_prefix("git@github.com:")
        .or_else(|| url.trim().strip_prefix("https://github.com/"))
        .or_else(|| url.trim().strip_prefix("ssh://git@github.com/"))?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let (owner, repo) = rest.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/'))
        .then(|| (owner.to_string(), repo.to_string()))
}

fn current_branch(cwd: &Path) -> String {
    run_quiet(cwd, "git", &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|b| !b.is_empty() && b != "HEAD")
        .unwrap_or_else(|| "main".to_string())
}

fn kaggle_configured() -> bool {
    if std::env::var_os("KAGGLE_USERNAME").is_some() && std::env::var_os("KAGGLE_KEY").is_some() {
        return true;
    }
    if std::env::var_os("KAGGLE_API_TOKEN").is_some() {
        return true;
    }
    let Some(home) = dirs_home() else {
        return false;
    };
    home.join(".kaggle/kaggle.json").exists() || home.join(".config/kaggle/kaggle.json").exists()
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
}

/// `/connect`: what Fikra Code can reach from this machine and folder.
pub(crate) fn connection_report(cwd: &Path) -> String {
    let mut out = String::from("Connections\n");
    let gh = run_quiet(cwd, "gh", &["--version"]).is_some();
    let gh_auth = gh && run_quiet(cwd, "gh", &["auth", "status"]).is_some();
    let remote = github_remote(cwd);
    out.push_str(&match (gh, gh_auth) {
        (true, true) => "  GitHub   connected (gh signed in)".to_string(),
        (true, false) => "  GitHub   gh installed, not signed in: run `gh auth login`".to_string(),
        _ => "  GitHub   install the GitHub CLI: https://cli.github.com".to_string(),
    });
    if let Some((owner, repo)) = &remote {
        out.push_str(&format!("  ·  repo {owner}/{repo}"));
    }
    out.push('\n');
    out.push_str(if kaggle_configured() {
        "  Kaggle   connected (API token found)\n"
    } else {
        "  Kaggle   add a token: kaggle.com/settings -> API -> Create New Token, save to ~/.kaggle/kaggle.json, then `pip install kaggle`\n"
    });
    out.push_str(&match &remote {
        Some((owner, repo)) => format!(
            "  Colab    ready: notebooks in {owner}/{repo} open at colab.research.google.com/github/{owner}/{repo}/blob/<branch>/<path>\n"
        ),
        None => "  Colab    needs a GitHub remote: notebooks open in Colab from GitHub\n".to_string(),
    });
    out.push_str("\nCommands: /pr, /issue <number>, /kaggle <dataset or competition>, /colab [notebook]");
    out
}

pub(crate) fn pr_prompt(args: &str) -> String {
    let title = args.trim();
    let title_line = if title.is_empty() {
        "Write a clear title from the changes.".to_string()
    } else {
        format!("Use this title: {title}")
    };
    format!(
        "Open a GitHub pull request for my current changes.\n\
         1. Run `git status` and `git diff` to see what changed. If nothing changed, tell me and stop.\n\
         2. If I'm on the default branch, create a descriptive branch first.\n\
         3. Run the project's tests or checks if they exist, and fix failures you introduced.\n\
         4. Commit with a concise message, push the branch, and open the PR with `gh pr create`. {title_line} \
         The description must summarise what changed, why, and how it was tested.\n\
         5. Reply with the PR URL and a two-line summary.\n\
         Ask before force-pushing or touching branches you did not create."
    )
}

pub(crate) fn issue_prompt(args: &str) -> String {
    let issue = args.trim();
    if issue.is_empty() {
        return "List my open GitHub issues for this repository with `gh issue list --assignee @me` \
                (or all open issues if none are assigned), and suggest which one to take first and why."
            .to_string();
    }
    format!(
        "Work on GitHub issue {issue}.\n\
         1. Read it with `gh issue view {issue} --comments`.\n\
         2. Find the relevant code, reproduce the problem or confirm the request, and make the change on a new branch.\n\
         3. Add or update tests, and run them.\n\
         4. Push and open a PR that says `Fixes {issue}` with `gh pr create`.\n\
         5. Reply with the PR URL and what you verified."
    )
}

pub(crate) fn kaggle_prompt(args: &str) -> String {
    let target = args.trim();
    if target.is_empty() {
        return "Help me with Kaggle from this folder. Check the Kaggle CLI works (`kaggle --version`; install with \
                `pip install kaggle` if missing), then ask me which dataset or competition to use."
            .to_string();
    }
    format!(
        "Set up Kaggle work for `{target}`.\n\
         1. Make sure the Kaggle CLI works (`kaggle --version`; `pip install kaggle` if missing). If no API token is configured, \
         tell me how to add one and stop.\n\
         2. If `{target}` is a competition, run `kaggle competitions download -c <name>`; if it is a dataset (owner/name), \
         run `kaggle datasets download -d {target} --unzip` into ./data.\n\
         3. Inspect the files: shapes, columns, missing values, target distribution.\n\
         4. Create a starter notebook `kaggle_{slug}.ipynb` with loading, a quick EDA and a simple baseline, and run what you can locally.\n\
         5. Summarise the data and the baseline result.",
        slug = target.replace(['/', ' '], "_")
    )
}

pub(crate) fn colab_prompt(args: &str, cwd: &Path) -> String {
    let notebook = args.trim();
    let link = match github_remote(cwd) {
        Some((owner, repo)) => format!(
            "https://colab.research.google.com/github/{owner}/{repo}/blob/{branch}/<path-to-notebook>",
            branch = current_branch(cwd)
        ),
        None => "https://colab.research.google.com/github/<owner>/<repo>/blob/<branch>/<path-to-notebook>".to_string(),
    };
    let what = if notebook.is_empty() {
        "Pick the most useful script or notebook in this project (ask me if unsure) and make it Colab-ready.".to_string()
    } else {
        format!("Make `{notebook}` Colab-ready (convert it to a notebook first if it is a script).")
    };
    format!(
        "{what}\n\
         - Add a first cell that installs dependencies with `%pip install ...` and, if needed, clones this repo.\n\
         - Make paths relative to the notebook, keep outputs cleared, and add an \"Open in Colab\" badge at the top.\n\
         - Commit and push it, then give me the Colab link in this form: {link}\n\
         - If there is no GitHub remote, tell me how to add one instead of guessing a URL."
    )
}

#[cfg(test)]
mod tests {
    use super::parse_github_remote;

    #[test]
    fn parses_github_remotes() {
        assert_eq!(
            parse_github_remote("git@github.com:Fikra-API/fikra-code.git"),
            Some(("Fikra-API".to_string(), "fikra-code".to_string()))
        );
        assert_eq!(
            parse_github_remote("https://github.com/Fikra-API/fikra-code"),
            Some(("Fikra-API".to_string(), "fikra-code".to_string()))
        );
        assert_eq!(parse_github_remote("https://gitlab.com/a/b.git"), None);
    }
}
