use deno_core::v8;
use hashbrown::HashMap;
use std::cell::RefCell;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum CollectorMode {
  #[default]
  Run,
  Skip,
  Only,
  Todo,
}

#[derive(Clone, Copy)]
pub enum CollectorState {
  Custom(CollectorMode),
  Fail,
  Pass,
}

impl From<String> for CollectorMode {
  fn from(value: String) -> Self {
    match value.as_str() {
      "run" => CollectorMode::Run,
      "skip" => CollectorMode::Skip,
      "only" => CollectorMode::Only,
      "todo" => CollectorMode::Todo,
      _ => {
        panic!("Invalid CollectorRunMode variant: '{}'", value)
      }
    }
  }
}

#[derive(Default, Clone)]
pub enum CollectorIdentifier {
  #[default]
  File,
  Custom(String),
}

impl std::fmt::Debug for CollectorIdentifier {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    const FILE_IDENT: &'static str = "$$file";

    let identifier = match self {
      CollectorIdentifier::Custom(e) => &e,
      CollectorIdentifier::File => FILE_IDENT,
    };

    write!(f, "{identifier:?}")
  }
}

#[derive(Clone)]
pub struct NodeCollectorManager {
  task_queue: Vec<Rc<CollectorTask>>,
  collector_node: Rc<CollectorNode>,
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
    let task_queue: Vec<Rc<CollectorTask>> = Vec::new();
    let collector_node = Rc::new(CollectorNode {
      identifier,
      mode: RefCell::new(mode),
      ..CollectorNode::default()
    });

    NodeCollectorManager {
      collector_node,
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

  #[must_use]
  pub fn collect_node(
    &mut self,
    collector_file: Rc<CollectorFile>,
  ) -> Option<Rc<CollectorNode>> {
    self.should_collect().then(|| {
      self.has_collected = true;

      *self.collector_node.file.borrow_mut() = Rc::downgrade(&collector_file);
      let tasks_queue = self.task_queue.clone();

      let tasks = tasks_queue
        .into_iter()
        .map(|task| {
          *task.node.borrow_mut() = Rc::downgrade(&self.collector_node);
          *task.file.borrow_mut() = Rc::downgrade(&collector_file);

          task
        })
        .collect();

      *self.collector_node.tasks.borrow_mut() = tasks;

      Rc::clone(&self.collector_node)
    })
  }

  pub fn register_task(
    &mut self,
    name: String,
    callback: TestCallback,
    mode: CollectorMode,
  ) {
    let created_task = Rc::new(CollectorTask::new(name, callback, mode));
    self.task_queue.push(created_task);
  }

  pub fn register_lifetime_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: TestCallback,
  ) {
    let hook_manager = &self.collector_node.hook_manager;
    hook_manager.borrow_mut().add_hook(hook_key, callback)
  }

  pub fn reset_state(&mut self) {
    self
      .on_file_level
      .then(|| {
        self.task_queue.clear();
        self.has_collected = false;

        let node = &self.collector_node;
        let identifier = node.identifier.clone();
        let mode = node.mode.clone();

        self.collector_node = Rc::new(CollectorNode {
          identifier,
          mode,
          ..CollectorNode::default()
        });
      })
      .unwrap_or_else(|| {
        panic!("Resetting state is only allowed when on_file_level is true.")
      })
  }

  pub fn get_node_factory(&self) -> &Option<TestCallback> {
    &self.node_factory
  }
}

#[derive(Default)]
pub struct CollectorFile {
  pub file_path: PathBuf,
  pub collected: RefCell<bool>,
  pub nodes: RefCell<Vec<Rc<CollectorNode>>>,
}

// temporary
impl std::fmt::Debug for CollectorFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorFile")
      .field("file", &self.file_path)
      .field("collected", &self.collected.borrow())
      .field("nodes", &self.nodes.borrow().iter().map(|n| n))
      .finish()
  }
}

#[derive(Default)]
pub struct CollectorNode {
  pub(crate) identifier: CollectorIdentifier,
  pub(crate) mode: RefCell<CollectorMode>,
  pub(crate) tasks: RefCell<Vec<Rc<CollectorTask>>>,
  file: RefCell<Weak<CollectorFile>>,
  status: Option<CollectorState>,
  hook_manager: RefCell<LifetimeHookManager>,
}

impl std::fmt::Debug for CollectorNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorNode")
      .field("name", &self.identifier)
      .field("mode", &self.mode.borrow())
      .field("tasks", &self.tasks.borrow().iter().map(|n| n))
      .finish()
  }
}

type TestCallback = v8::Global<v8::Function>;

// TODO think about making whole struct
// RefCell instead of fields
pub struct CollectorTask {
  pub(crate) name: String,
  pub(crate) mode: RefCell<CollectorMode>,
  pub(crate) state: RefCell<CollectorState>,
  node: RefCell<Weak<CollectorNode>>,
  file: RefCell<Weak<CollectorFile>>,
  callback: TestCallback,
}

impl std::fmt::Debug for CollectorTask {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorTask")
      .field("name", &self.name)
      .field("mode", &self.mode)
      .finish()
  }
}

impl CollectorTask {
  pub fn new(
    name: String,
    callback: TestCallback,
    mode: CollectorMode,
  ) -> Self {
    CollectorTask {
      name,
      mode: RefCell::new(mode),
      file: RefCell::new(Weak::new()),
      node: RefCell::new(Weak::new()),
      state: RefCell::new(CollectorState::Custom(mode)),
      callback,
    }
  }
}

pub struct LifetimeHookManager {
  hooks: HashMap<LifetimeHook, Vec<TestCallback>>,
}

impl LifetimeHookManager {
  pub fn new() -> Self {
    let mut hooks: HashMap<LifetimeHook, Vec<TestCallback>> = HashMap::new();

    hooks.insert(LifetimeHook::BeforeAll, Vec::new());
    hooks.insert(LifetimeHook::AfterAll, Vec::new());
    hooks.insert(LifetimeHook::BeforeEach, Vec::new());
    hooks.insert(LifetimeHook::AfterEach, Vec::new());

    LifetimeHookManager { hooks }
  }

  pub fn add_hook(&mut self, hook_key: LifetimeHook, callback: TestCallback) {
    self
      .hooks
      .get_mut(&hook_key)
      .and_then(|partition| {
        partition.push(callback);
        Some(partition)
      })
      .unwrap_or_else(|| panic!("wrong lifetime hook method"));
  }
}

impl Default for LifetimeHookManager {
  fn default() -> Self {
    LifetimeHookManager::new()
  }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum LifetimeHook {
  BeforeAll,
  AfterAll,
  BeforeEach,
  AfterEach,
}

impl From<String> for LifetimeHook {
  fn from(value: String) -> Self {
    match value.as_str() {
      "beforeAll" => LifetimeHook::BeforeAll,
      "afterAll" => LifetimeHook::AfterAll,
      "beforeEach" => LifetimeHook::BeforeEach,
      "afterEach" => LifetimeHook::AfterEach,
      _ => unreachable!(),
    }
  }
}
