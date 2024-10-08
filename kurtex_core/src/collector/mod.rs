use std::sync::{Arc, Mutex};

use crate::arc_mut;
pub use context::*;
pub use structures::*;

pub mod context;
pub mod structures;

pub struct NodeCollectorManager {
  task_queue: Vec<Arc<Mutex<CollectorTask>>>,
  inner_node: Arc<Mutex<CollectorNode>>,
  has_collected: bool,
  node_factory: Option<TestCallback>,
  on_file_level: bool,
}

impl NodeCollectorManager {
  pub fn new_with_file() -> Self {
    NodeCollectorManager {
      on_file_level: true,
      ..Self::new(CollectorIdentifier::File, CollectorMode::Run, None)
    }
  }

  pub fn new(
    identifier: CollectorIdentifier,
    mode: CollectorMode,
    node_factory: Option<TestCallback>,
  ) -> Self {
    let task_queue = Vec::new();
    let collector_node = arc_mut!(CollectorNode {
      identifier,
      mode,
      ..CollectorNode::default()
    });

    NodeCollectorManager {
      inner_node: collector_node,
      task_queue,
      has_collected: false,
      on_file_level: false,
      node_factory,
    }
  }

  pub fn new_with_factory(
    identifier: CollectorIdentifier,
    mode: CollectorMode,
    factory: TestCallback,
  ) -> Self {
    Self::new(identifier, mode, Some(factory))
  }

  #[inline]
  #[must_use]
  fn should_collect(&self) -> bool {
    !self.has_collected
  }

  #[inline]
  #[must_use]
  pub fn collect_node(&mut self) -> Arc<Mutex<CollectorNode>> {
    self
      .should_collect()
      .then(|| {
        self.has_collected = true;
        let tasks_queue = self.task_queue.clone();

        {
          let mut inner_node = self.inner_node.lock().unwrap();
          inner_node.tasks = tasks_queue;
        }

        self.inner_node.clone()
      })
      .unwrap_or_else(|| {
        let inner_node = self.inner_node.lock().unwrap();

        panic!(
          "File ({node}) CollectorNode has been already collected.",
          node = format!("{:?}", inner_node.identifier)
        )
      })
  }

  pub fn register_task(
    &mut self,
    name: String,
    callback: TestCallback,
    mode: CollectorMode,
  ) {
    let created_task =
      Arc::new(Mutex::new(CollectorTask::new(name, callback, mode)));

    self.task_queue.push(created_task);
  }

  pub fn register_lifetime_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: TestCallback,
  ) {
    let mut collector_node = self.inner_node.lock().unwrap();

    collector_node.hook_manager.add_hook(hook_key, callback);
  }

  pub fn get_node_factory(&self) -> Option<TestCallback> {
    self.node_factory.as_ref().map(Clone::clone)
  }
}

impl Default for NodeCollectorManager {
  fn default() -> Self {
    NodeCollectorManager::new_with_file()
  }
}
