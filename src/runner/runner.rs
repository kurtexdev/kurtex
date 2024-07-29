use std::borrow::Cow;

use deno_core::error::AnyError;
use globwalk;

use crate::context::{ContextProvider, RUNTIME_CONFIG};
use crate::runtime::runtime::RuntimeConfig;
use crate::CLI_CONFIG;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

impl Runner {
  pub fn run_with_options() -> Result<(), AnyError> {
    let cli_config = ContextProvider::get(&CLI_CONFIG).unwrap();
    let runtime_config = ContextProvider::get(&RUNTIME_CONFIG).unwrap();

    // let RuntimeConfig { options: runtime_opts, root, .. } = runtime_config;

    if cli_config.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    Self::collect_test_files(&runtime_config)?.for_each(|file_path| {
      println!("file_path: {:?}", file_path);
    });

    Ok(())
  }

  fn collect_test_files(
    runtime_cfg: &RuntimeConfig,
  ) -> Result<impl Iterator<Item = std::path::PathBuf> + '_, AnyError> {
    let runtime_opts = &runtime_cfg.options;
    let walk_glob = |patterns: &Vec<Cow<'static, str>>| {
      globwalk::GlobWalkerBuilder::from_patterns(&runtime_cfg.root, patterns)
        .max_depth(TEST_FILES_MAX_DEPTH as usize)
        .build()
        .unwrap()
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
    };

    #[allow(unused_mut)]
    let mut included_cases = walk_glob(&runtime_opts.includes);
    let mut excluded_cases = walk_glob(&runtime_opts.excludes);

    Ok(included_cases.filter(move |included_path| {
      !excluded_cases.any(|excluded_path| excluded_path.eq(included_path))
    }))
  }

  pub fn run() {}
}
