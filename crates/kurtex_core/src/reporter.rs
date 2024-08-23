use std::sync::{Arc, Mutex};
use std::time;

use anyhow::{anyhow, bail};
use log::debug;
use nu_ansi_term::Color::{
  LightBlue, LightGray, LightGreen, LightYellow, Red, White,
};
use nu_ansi_term::{Color, Style};
use rccell::RcCell;

use crate::runner::collector::RunnerCollectorContext;
use crate::{
  CollectorFile, CollectorMode, CollectorNode, CollectorStatus, CollectorTask,
};

pub struct KurtexDefaultReporter {
  start_time: time::Instant,
  // context: RcCell<RunnerCollectorContext>,
}

// TODO: better namings
pub trait Reporter {
  fn start(&self) {}
  fn report_collected(&mut self);
  fn report_finished(&self, ctx: &RunnerCollectorContext);
  fn begin_file(&self, file: Arc<CollectorFile>) {}
  fn end_file(&self, file: Arc<CollectorFile>) {}
  fn begin_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn end_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn begin_task(&self, task: Arc<Mutex<CollectorTask>>) {}
  fn end_task(&self, task: Arc<Mutex<CollectorTask>>) {}
}

impl KurtexDefaultReporter {
  pub fn new() -> Self {
    let start_time = time::Instant::now();

    KurtexDefaultReporter { start_time }
  }
}

impl Reporter for KurtexDefaultReporter {
  fn report_collected(&mut self) {
    self.start_time = time::Instant::now();

    debug!("Reporter: collected test files.");
  }

  fn report_finished(&self, ctx: &RunnerCollectorContext) {
    let end_time = self.start_time.elapsed();
    let milliseconds = end_time.as_micros() as f64 / 1_000.0;

    // TODO: onFinished
    debug!("Reporter: finished with {}.", end_time.as_millis());
    println!();

    let mut failed_files = ctx
      .files
      .iter()
      .filter(|file| file.error.is_some())
      .collect::<Vec<&Arc<CollectorFile>>>();

    let mut failed = task_vec![];
    let mut passed = task_vec![];
    let mut runnable = task_vec![];
    let mut skipped = task_vec![];
    let mut todo = task_vec![];

    fn paint(color: nu_ansi_term::Color, msg: String) {
      println!("{}", color.paint(msg))
    };

    fn paint_if<T>(failed: &Vec<T>, color: nu_ansi_term::Color, msg: String) {
      if !failed.is_empty() {
        println!("{}", color.paint(msg))
      }
    };

    let tasks = ctx.tasks.iter();

    for task_rc in tasks {
      let task = task_rc.lock().unwrap();
      let is_runnable =
        matches!(task.status, CollectorStatus::Pass | CollectorStatus::Fail);

      match task.status {
        CollectorStatus::Fail => failed.push(task_rc.clone()),
        CollectorStatus::Pass => passed.push(task_rc.clone()),
        CollectorStatus::Custom(CollectorMode::Skip) => {
          skipped.push(task_rc.clone())
        }
        CollectorStatus::Custom(CollectorMode::Todo) => {
          todo.push(task_rc.clone())
        }
        _ => {}
      };

      if is_runnable {
        runnable.push(task_rc.clone());
      }
    }

    let print_failed_files = || {
      let has_failed = !failed_files.is_empty();

      has_failed.then(|| {
        println!("{}", format!("Failed to parse {} files", failed_files.len()));

        failed_files.iter().for_each(|file| {
          let file_path = file.file_path.display().to_string();
          let error = file.error.as_ref().map(|e| e.to_string());

          println!();
          paint(Red, format!("{}", file_path));
          eprintln!("{}", error.unwrap());
          println!();
        });
      })
    };

    let print_failed_tasks = || {
      let has_failed = !failed.is_empty();

      has_failed.then(|| {
        println!("{}", format!("Failed tests ({})", failed.len()));

        failed.iter().for_each(|task| {
          let task = task.lock().unwrap();
          let error = task.error.as_ref().map(|e| e.to_string());

          let bold_red = Style::new().bold().on(Red);
          let fail_mark = format!(" {} ", bold_red.paint("FAIL"));

          println!();
          println!("{} {}", fail_mark, task.name);
          eprintln!("{}", error.unwrap());
          println!();
        });
      })
    };

    print_failed_files();
    print_failed_tasks();

    paint_if(
      &failed_files,
      White,
      format!("Failed to parse {} files", failed_files.len()),
    );

    paint_if(
      &failed,
      Red,
      format!("Failed {} / {}", failed.len(), runnable.len()),
    );

    paint(LightGreen, format!("Passed {} / {}", passed.len(), runnable.len()));

    paint_if(&skipped, LightYellow, format!("Skipped  {}", skipped.len()));

    paint_if(&todo, White, format!("Todo  {} ", todo.len()));

    println!("Time {}ms", milliseconds);
  }
}

impl Default for KurtexDefaultReporter {
  fn default() -> Self {
    KurtexDefaultReporter::new()
  }
}

#[macro_export]
macro_rules! create_task_vector {
  ($($tt:tt)*) => {{
    use crate::CollectorTask;

    let result: Vec<::std::sync::Arc<::std::sync::Mutex<CollectorTask>>> =
      Vec::new();
    result
  }};
}

pub use create_task_vector as task_vec;
