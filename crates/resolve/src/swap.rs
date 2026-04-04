use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::reporter::Reporter;

const STAGING_SUFFIX: &str = "_staging";
const PREVIOUS_SUFFIX: &str = "_previous";
const JOURNAL_SUFFIX: &str = "_swap_journal.json";

#[derive(Debug, Serialize, Deserialize)]
struct SwapJournal {
    staging: String,
    output: String,
    previous: String,
}

pub fn staging_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();

    assert!(
        !name.is_empty(),
        "output directory must have a name component",
    );

    let staging = format!("{}{}", name, STAGING_SUFFIX);

    let result = match output.parent() {
        Some(parent) => parent.join(staging),
        None => PathBuf::from(staging),
    };

    assert!(
        result != output,
        "staging directory must differ from output directory",
    );

    result
}

fn previous_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();

    assert!(
        !name.is_empty(),
        "output directory must have a name component for previous_path",
    );

    let previous = format!("{}{}", name, PREVIOUS_SUFFIX);

    let result = match output.parent() {
        Some(parent) => parent.join(&previous),
        None => PathBuf::from(&previous),
    };

    assert!(
        !result.as_os_str().is_empty(),
        "previous_path must produce a non-empty path",
    );

    result
}

fn journal_path(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();

    assert!(
        !name.is_empty(),
        "output directory must have a name component for journal_path",
    );

    let journal = format!("{}{}", name, JOURNAL_SUFFIX);

    let result = match output.parent() {
        Some(parent) => parent.join(&journal),
        None => PathBuf::from(&journal),
    };

    assert!(
        !result.as_os_str().is_empty(),
        "journal_path must produce a non-empty path",
    );

    result
}

fn write_journal(path: &Path, journal: &SwapJournal) -> Result<()> {
    assert!(
        !journal.staging.is_empty(),
        "journal staging path must not be empty",
    );

    assert!(
        !journal.output.is_empty(),
        "journal output path must not be empty",
    );

    let content = serde_json::to_string(journal)?;
    let temp = path.with_extension("tmp");

    let mut file = fs::File::create(&temp)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;

    fs::rename(&temp, path)?;

    assert!(
        path.exists(),
        "journal file must exist after write: {:?}",
        path,
    );

    Ok(())
}

fn remove_journal(path: &Path) -> Result<()> {
    assert!(
        !path.as_os_str().is_empty(),
        "journal path must not be empty",
    );

    if path.exists() {
        fs::remove_file(path)?;

        assert!(
            !path.exists(),
            "journal file must not exist after removal: {:?}",
            path,
        );
    }

    Ok(())
}

fn cleanup(directory: &Path) -> Result<()> {
    assert!(
        !directory.as_os_str().is_empty(),
        "cleanup directory path must not be empty",
    );

    if directory.exists() {
        fs::remove_dir_all(directory)?;

        assert!(
            !directory.exists(),
            "directory must not exist after cleanup: {:?}",
            directory,
        );
    }

    Ok(())
}

pub fn recover_interrupted(output: &Path, reporter: &Reporter) -> Result<()> {
    assert!(
        !output.as_os_str().is_empty(),
        "output must not be empty for recovery",
    );

    let journal = journal_path(output);

    if !journal.exists() {
        return Ok(());
    }

    assert!(
        journal.is_file(),
        "journal path must be a file if it exists: {:?}",
        journal,
    );

    reporter.info("Recovering interrupted swap...");

    let content = fs::read_to_string(&journal)?;
    let entry: SwapJournal = serde_json::from_str(&content)?;

    let output = Path::new(&entry.output);
    let previous = Path::new(&entry.previous);
    let staging = Path::new(&entry.staging);

    if !output.exists() && previous.exists() {
        reporter.info("  Rolling back: previous -> output");
        fs::rename(previous, output)?;
    }

    cleanup(staging)?;

    if previous.exists() {
        cleanup(previous)?;
    }

    remove_journal(&journal)?;

    reporter.info("  Recovery complete");

    Ok(())
}

pub fn swap_staging_to_output(staging: &Path, output: &Path, reporter: &Reporter) -> Result<()> {
    assert!(
        staging.exists(),
        "staging directory must exist before swap: {:?}",
        staging,
    );

    assert!(
        staging != output,
        "staging and output directories must differ",
    );

    let previous = previous_path(output);
    let journal = journal_path(output);

    cleanup(&previous)?;

    let entry = SwapJournal {
        staging: staging.to_string_lossy().into_owned(),
        output: output.to_string_lossy().into_owned(),
        previous: previous.to_string_lossy().into_owned(),
    };

    write_journal(&journal, &entry)?;

    if output.exists() {
        fs::rename(output, &previous)?;
    }

    fs::rename(staging, output)?;

    cleanup(&previous)?;
    remove_journal(&journal)?;

    assert!(
        output.exists(),
        "output directory must exist after swap: {:?}",
        output,
    );

    assert!(
        !staging.exists(),
        "staging directory must not exist after swap: {:?}",
        staging,
    );

    reporter.debug(&format!(
        "Atomic swap complete: {} -> {}",
        staging.display(),
        output.display(),
    ));

    Ok(())
}
