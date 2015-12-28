use iron_llvm::core;
use iron_llvm::core::value::{FunctionRef, Function};

use jitter::JITter;

use llvm_sys::prelude::LLVMValueRef;

pub trait ModuleProvider {
  fn dump(&self);
  fn get_module(&mut self) -> &mut core::Module;
  fn get_function(&mut self, name: &str) -> Option<(FunctionRef, bool)>;
  fn get_pass_manager(&mut self) -> &mut core::FunctionPassManager;
}

pub struct SimpleModuleProvider {
  module: core::Module,
  func_pass_manager: core::FunctionPassManager,
}

impl SimpleModuleProvider {
  pub fn new(name: &str) -> SimpleModuleProvider {
    let (module, func_pass_manager) = new_module(name);
    SimpleModuleProvider {
      module: module,
      func_pass_manager: func_pass_manager
    }
  }
}

impl JITter for SimpleModuleProvider {
  fn get_module_provider(&mut self) -> &mut ModuleProvider {
    self
  }

  fn run_function(&mut self, _f: LLVMValueRef) -> f64 {
    panic!("not implemented")
  }
}

impl ModuleProvider for SimpleModuleProvider {
  fn dump(&self) {
    self.module.dump();
  }

  fn get_module(&mut self) -> &mut core::Module {
    &mut self.module
  }

  fn get_function(&mut self, name: &str) -> Option<(FunctionRef, bool)> {
    match self.module.get_function_by_name(name) {
      Some(f) => Some((f, f.count_basic_blocks() > 0)), // f.count_basic_blocks() > 0 means a function of the given name is already defined in some basic blocks.
      None => None
    }
  }

  fn get_pass_manager(&mut self) -> &mut core::FunctionPassManager {
    &mut self.func_pass_manager
  }
}

pub fn new_module(name: &str) -> (core::Module, core::FunctionPassManager) {
  let module = core::Module::new(name);
  let mut function_passmanager = core::FunctionPassManager::new(&module);
  function_passmanager.add_basic_alias_analysis_pass();
  function_passmanager.add_instruction_combining_pass();
  function_passmanager.add_reassociate_pass();
  function_passmanager.add_GVN_pass();
  function_passmanager.add_CFG_simplification_pass();
  function_passmanager.initialize();

  (module, function_passmanager)
}
