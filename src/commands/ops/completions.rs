// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::Path;

use crate::cli::CompletionShell;
use crate::error::{CliError, ErrorKind};

pub(crate) fn handle_completions(shell: CompletionShell) -> Result<(), CliError> {
    use clap::CommandFactory;
    use clap_complete::{generate, shells};
    use std::io::Write;

    let mut cmd = crate::cli::Cli::command();
    let bin = crate::constants::PRODUCT_BIN_NAME;
    let mut out = std::io::stdout();
    match shell {
        CompletionShell::Bash => generate(shells::Bash, &mut cmd, bin, &mut out),
        CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, bin, &mut out),
        CompletionShell::Fish => generate(shells::Fish, &mut cmd, bin, &mut out),
        CompletionShell::Elvish => generate(shells::Elvish, &mut cmd, bin, &mut out),
        CompletionShell::Powershell => generate(shells::PowerShell, &mut cmd, bin, &mut out),
    }
    let _ = out.flush();
    Ok(())
}

/// Render man page (roff) with clap_mangen to stdout or `--out PATH`.
pub(crate) fn handle_man(out: Option<&Path>) -> Result<(), CliError> {
    use clap::CommandFactory;
    use std::io::Write;

    let cmd = crate::cli::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("manpage render failed: {e}")))?;

    if let Some(path) = out {
        crate::validation::reject_path_traversal(path).map_err(|m| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                m,
                crate::i18n::suggestion_key("path_no_parent_components", None),
            )
        })?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("create man parent: {e}")))?;
            }
        }
        // Atomic write: temp + rename beside destination when possible.
        let tmp = path.with_extension("1.tmp");
        std::fs::write(&tmp, &buf)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("write man temp: {e}")))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            CliError::new(ErrorKind::Io, format!("rename man page: {e}"))
        })?;
    } else {
        let mut stdout = std::io::stdout();
        stdout
            .write_all(&buf)
            .map_err(|e| CliError::new(ErrorKind::BrokenPipe, format!("stdout: {e}")))?;
        let _ = stdout.flush();
    }
    Ok(())
}
