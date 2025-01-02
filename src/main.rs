use std::{fs::File, io::BufReader, process::Stdio, sync::Arc};
use tokio::{process::Command, task::JoinSet};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

type Error = Box<dyn std::error::Error>;

const RUNNER_JSON: &str = "runner.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    name: String,
    cmd: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Runner {
    tasks: Option<Vec<Task>>,
    builds: Option<Vec<Task>>,
}

struct InternalRunner {
    tasks: Option<Arc<tokio::sync::Mutex<Vec<Task>>>>,
    builds: Option<Arc<tokio::sync::Mutex<Vec<Task>>>>,
}

impl InternalRunner {
    async fn build_all(&self) {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();

        println!("Building all builds\n");
        let mut set = JoinSet::new();

        if let Some(builds) = &self.builds {
            for build in builds.lock().await.iter() {
                self.run_task(build.clone(), cancellation_token.clone(), &mut set)
                    .await;
            }
        }

        ctrlc::set_handler(move || {
            println!("Exiting...");
            cancellation_token.cancel();
        })
        .expect("Error setting Ctrl-C handler");

        while let Some(_) = set.join_next().await {}

        // everything is done
        _cancellation_token.cancel();
    }

    async fn run_all(&self) {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let mut set = JoinSet::new();

        println!("Running all tasks\n");

        if let Some(tasks) = &self.tasks {
            for task in tasks.lock().await.iter() {
                self.run_task(task.clone(), cancellation_token.clone(), &mut set)
                    .await;
            }
        }

        ctrlc::set_handler(move || {
            println!("Exiting...");
            cancellation_token.cancel();
        })
        .expect("Error setting Ctrl-C handler");

        while let Some(_) = set.join_next().await {}

        // everything is done
        _cancellation_token.cancel();
    }

    async fn run_task(
        &self,
        task: Task,
        _cancellation_token: CancellationToken,
        set: &mut JoinSet<()>,
    ) {
        let _cancellation_token = _cancellation_token.clone();
        set.spawn(async move {
            loop {
                let rate = tokio::time::Duration::from_millis(1000);
                tokio::time::sleep(rate).await;
                tokio::select! {
                    _ = _cancellation_token.cancelled() => {
                        println!("Task cancelled {}", task.name);
                        break;
                    },
                    _ = run_task_inner(task.clone()) => {
                        break;
                    }
                }
            }
        });
    }
}

async fn run_task_inner(task: Task) {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&task.cmd)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    println!("Running task: \"{}\"", task.name);

    let output = cmd.output().await.expect("failed to execute process");

    if output.status.success() {
        println!("\nTask: \"{}\" succeeded", task.name);
        println!("Task succeeded");
        println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("\nTask \"{}\" failed", task.name);
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}

impl Runner {
    fn init() -> Result<InternalRunner, Error> {
        if let Ok(runner) = Runner::init_runner(RUNNER_JSON) {
            let tasks = if let Some(tasks) = runner.tasks {
                Some(Arc::new(tokio::sync::Mutex::new(tasks)))
            } else {
                None
            };

            let builds = if let Some(builds) = runner.builds {
                Some(Arc::new(tokio::sync::Mutex::new(builds)))
            } else {
                None
            };

            Ok(InternalRunner { tasks, builds })
        } else {
            Err("Failed to initialize runner".into())
        }
    }

    fn init_runner(runner_str: &str) -> Result<Runner, Error> {
        let file = File::open(runner_str)?;
        let reader = BufReader::new(file);

        Ok(serde_json::from_reader(reader)?)
    }
}

#[derive(Subcommand)]
enum Commands {
    Run,
    R,
    Build,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    fn print_version() {
        println!(
            "{} initialized v{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
    }
}

#[tokio::main]
async fn main() {
    if let Ok(runner) = Runner::init() {
        let cli = Cli::parse();
        Cli::print_version();

        match cli.command {
            Some(Commands::R) => {
                runner.run_all().await;
            }
            Some(Commands::Run) => {
                runner.run_all().await;
            }
            Some(Commands::Build) => {
                runner.build_all().await;
            }
            None => {
                println!("No command specified");
            }
        }
    }
}
