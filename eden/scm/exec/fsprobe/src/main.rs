/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{bail, Result};
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::TrySendError;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Cli {
    path: PathBuf,
    #[structopt(short = "p", long = "parallel")]
    parallel: Option<usize>,
}

fn main() {
    let args = Cli::from_args();
    let plan = ProbePlan::load(&args.path).expect("Failed to load fsprobe plan");
    let stats = Arc::new(Stats::default());
    let start = Instant::now();
    if let Some(threads) = args.parallel {
        plan.run_parallel(&stats, threads);
    } else {
        plan.run(&stats);
    }
    let duration = Instant::now() - start;
    let duration_ms = duration.as_millis() as f64;
    let rate = rate(stats.bytes.load(Ordering::Relaxed) as f64 / (duration_ms / 1000.));
    let files = stats.files.load(Ordering::Relaxed) as f64;
    let lat = duration_ms / files;
    let qps = files / duration_ms;
    println!(
        "lat: {:.4} ms, qps: {:.0}, dur: {:?}, {}, rate {}",
        lat, qps, duration, stats, rate
    );
}

#[derive(Default)]
struct Stats {
    files: AtomicU64,
    bytes: AtomicU64,
    errors: AtomicU64,
}

struct ProbePlan(Vec<ProbeAction>);

enum ProbeAction {
    Read(PathBuf),
}

// Probe plan file format is a new line separated list of actions
// Each action has a format <action> [<params>]
// Currently supported actions:
//   * cat <path> - read full file at <path>
impl ProbePlan {
    fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mut actions = vec![];
        for line in BufReader::new(file).lines() {
            let line = line?;
            let action = ProbeAction::parse(&line)?;
            actions.push(action);
        }
        Ok(Self(actions))
    }

    pub fn run(self, stats: &Arc<Stats>) {
        for action in self.0 {
            action.run(&*stats);
        }
    }

    pub fn run_parallel(self, stats: &Arc<Stats>, thread_count: usize) {
        let mut threads = vec![];
        let mut senders = vec![];
        for _ in 0..thread_count {
            let stats = stats.clone();
            let (sender, recv) = mpsc::sync_channel::<ProbeAction>(8);
            let thread = thread::spawn(move || {
                for action in recv {
                    action.run(&*stats)
                }
            });
            threads.push(thread);
            senders.push(sender);
        }
        let mut idx = 0;
        for mut action in self.0 {
            loop {
                idx = (idx + 1) % senders.len();
                match senders[idx].try_send(action) {
                    Ok(_) => break,
                    Err(TrySendError::Disconnected(_)) => panic!("Worker terminated"),
                    Err(TrySendError::Full(ret)) => action = ret,
                }
            }
        }
        senders.clear();
        for thread in threads {
            thread.join().expect("Worker panic");
        }
    }
}

impl ProbeAction {
    pub fn parse(s: &str) -> Result<Self> {
        let space = s.find(' ');
        if let Some(space) = space {
            let cmd = &s[..space];
            match cmd {
                "cat" => {
                    let path = &s[space + 1..];
                    if path.len() == 0 {
                        bail!("cat requires path");
                    }
                    Ok(ProbeAction::Read(path.into()))
                }
                _ => bail!("Unknown command {}", cmd),
            }
        } else {
            bail!("Invalid action {}", s);
        }
    }

    pub fn run(&self, stats: &Stats) {
        let r = match self {
            Self::Read(path) => Self::read(path, stats),
        };
        if let Err(err) = r {
            stats.errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("{} failed: {}", self, err);
        }
    }

    fn read(path: &Path, stats: &Stats) -> Result<()> {
        let mut file = File::open(path)?;
        let mut v = vec![];
        file.read_to_end(&mut v)?;
        stats.bytes.fetch_add(v.len() as u64, Ordering::Relaxed);
        stats.files.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl fmt::Display for ProbeAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path) => write!(f, "cat {}", path.display()),
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} files, {} bytes, {} errors",
            self.files.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}

fn rate(rate: f64) -> String {
    // Guard against zero, NaN, infinity, etc.
    if !rate.is_normal() {
        return "0 b/s".into();
    }

    // Divide by the base-1000 log of the value to bring it under 1000.
    let log = (rate.log10() / 3.0).floor() as usize;
    let shifted = rate / 1000f64.powi(log as i32);

    // Determine unit and precision to display.
    let unit = ["b/s", "kb/s", "Mb/s", "Gb/s", "Tb/s", "Pb/s", "Eb/s"][log];
    let prec = if log > 1 { 2 } else { 0 };

    format!("{:.*} {}", prec, shifted, unit)
}
