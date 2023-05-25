mod cfg;
mod cli;
mod cmd;
mod snapshot;

use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Select};
use eyre::{eyre, Result};
use monostate::MustBe;
use owo_colors::OwoColorize;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use similar::{ChangeTag, TextDiff};

use std::{
    env::set_current_dir,
    ffi::OsStr,
    fs::{self, create_dir_all, read_dir, remove_dir_all, File},
    io::{stderr, BufRead, Write},
    path::{Path, PathBuf},
    process::exit,
};

use crate::{
    cli::{Opts, Subcommand},
    cmd::run,
    snapshot::Snapshot,
};

#[derive(Deserialize, Debug)]
struct TestOutput {
    dir: PathBuf,
    results: FxHashMap<String, TestResult>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TestResult {
    Success(MustBe!(true)),
    Failure {
        #[serde(flatten)]
        snapshot: Snapshot,
        old: bool,
    },
}

fn main() -> Result<()> {
    let _ = color_eyre::install();

    let opts = Opts::parse();
    if let Some(dir) = opts.dir {
        set_current_dir(dir)?;
    }

    let cfg = cfg::load()?;

    match opts.subcmd {
        Subcommand::Check => {
            let output = run(
                opts.cmd,
                (|| cfg?.check?.cmd)(),
                "nix",
                [
                    "flake",
                    "check",
                    "--extra-experimental-features",
                    "flakes nix-command",
                ],
            )?;

            let success = output.status.success();
            for line in output.stderr.lines() {
                let line = line?;
                let Some(line) = line.strip_prefix("trace: namaka=") else {
                        continue;
                    };

                let output = serde_json::from_str::<TestOutput>(line)?;

                let pending = output.dir.join("_snapshots").join(".pending");
                let _ = remove_dir_all(&pending);
                create_dir_all(&pending)?;
                fs::write(pending.join(".gitignore"), "*")?;

                let total = output.results.len();
                let mut failures = 0;
                for (name, res) in output.results {
                    let new = pending.join(&name);
                    match res {
                        TestResult::Success(_) => {
                            println!("{} {name}", "✔".green());
                        }

                        TestResult::Failure { snapshot, old } => {
                            failures += 1;
                            println!("{} {name}", if old { "✘" } else { "🞥" }.red());
                            snapshot.to_writer(File::create(new)?)?;
                        }
                    }
                }

                if failures == 0 {
                    if success {
                        eprintln!("All {total} tests succeeded");
                        return Ok(());
                    } else {
                        break;
                    }
                } else {
                    eprintln!("{failures} out of {total} tests failed");
                    eprintln!("run `namaka review` to review the pending snapshots");
                    exit(1);
                }
            }

            stderr().write_all(&output.stderr)?;
            Err(eyre!("unknown error"))
        }

        Subcommand::Review => {
            let output = run(
                opts.cmd,
                (|| cfg?.eval?.cmd)(),
                "nix",
                [
                    "eval",
                    ".#checks",
                    "--extra-experimental-features",
                    "flakes nix-command",
                ],
            )?;

            for line in output.stderr.lines() {
                let line = line?;
                let Some(line) = line.strip_prefix("trace: namaka=") else {
                    continue;
                };

                let output = serde_json::from_str::<TestOutput>(line)?;
                let snapshots = output.dir.join("_snapshots");

                for entry in read_dir(snapshots.join(".pending"))? {
                    use bstr::ByteSlice;

                    let entry = entry?;
                    let name = entry.file_name();

                    if <[u8] as ByteSlice>::from_os_str(&name)
                        .map_or(false, |name| name.starts_with(b"."))
                    {
                        continue;
                    };

                    let old = snapshots.join(&name);
                    let new = entry.path();
                    let new_snap = Snapshot::parse(File::open(&new)?)?;
                    println!();

                    if let Ok(old_snap) = File::open(&old) {
                        match (Snapshot::parse(old_snap), new_snap) {
                            (Ok(Snapshot::Json(old_value)), Snapshot::Json(new_value)) => {
                                print_diff(
                                    "json",
                                    &serde_json::to_string_pretty(&old_value)?,
                                    &serde_json::to_string_pretty(&new_value)?,
                                )?;
                            }
                            (Ok(Snapshot::Pretty(old_value)), Snapshot::Pretty(new_value)) => {
                                print_diff("pretty", &old_value, &new_value)?;
                            }
                            (Ok(Snapshot::String(old_value)), Snapshot::String(new_value)) => {
                                print_diff("string", &old_value, &new_value)?;
                            }
                            (Ok(old_snap), new_snap) => {
                                old_snap.print_old()?;
                                new_snap.print_new()?;
                            }
                            (Err(e), new_snap) => {
                                println!("  {} failed to parse: {e}", "old".bold().red());
                                new_snap.print_new()?;
                            }
                        }
                        ask(&name, &old, &new)?;
                    } else {
                        println!("  {}: N/A", "old".bold().red());
                        new_snap.print_new()?;
                        ask(&name, &old, &new)?;
                    }
                }

                return Ok(());
            }

            stderr().write_all(&output.stderr)?;
            Err(eyre!("unknown error"))
        }
    }
}

fn print_diff(fmt: &'static str, old: &str, new: &str) -> Result<()> {
    let diff = TextDiff::from_graphemes(old, new);
    for change in diff.iter_all_changes() {
        let tag = change.tag();
        let change = change.to_string_lossy();
        match tag {
            ChangeTag::Equal => print!("{change}"),
            ChangeTag::Delete => print!("{}", change.bold().red()),
            ChangeTag::Insert => print!("{}", change.bold().green()),
        }
    }

    if !diff.newline_terminated() {
        println!("⏎");
    }

    println!(
        "{}\n---",
        format_args!("({fmt}) {} | {}", "old".red(), "new".green()).bold(),
    );

    Ok(())
}

fn ask(name: &OsStr, old: &Path, new: &Path) -> Result<()> {
    let choice = Select::with_theme(&ColorfulTheme::default())
        .item("accept".green())
        .item("reject".red())
        .item("skip".blue())
        .default(0)
        .with_prompt(format!("Review {}", name.to_string_lossy()))
        .interact()?;

    match choice {
        0 => fs::rename(new, old).map_err(Into::into),
        1 => fs::remove_file(new).map_err(Into::into),
        2 => Ok(()),
        _ => Err(eyre!("invalid selection")),
    }
}
