use std;
use std::rc::Rc;
use std::cell::RefCell;

use iron_llvm::{LLVMRefCtor, LLVMRef};
use iron_llvm::core;
use iron_llvm::core::value::{Function, FunctionRef, Value, FunctionCtor};
use iron_llvm::core::types::{RealTypeRef, RealTypeCtor, FunctionType, FunctionTypeRef};
use iron_llvm::execution_engine::{BindingSectionMemoryManagerBuilder, ExecutionEngine, MCJITBuilder};
use iron_llvm::execution_engine::execution_engine::FrozenModule;
use iron_llvm::support::add_symbol;

use module;
use module::ModuleProvider;

use llvm_sys::prelude::LLVMValueRef;

pub extern fn printd(x: f64) -> f64 {
  println!("> {} <", x);
  x
}

pub extern fn putchard(x: f64) -> f64 {
  print!("{}", x as u8 as char);
  x
}

pub fn init() {
  unsafe {
    add_symbol("printd", printd as *const ());
    add_symbol("putchard", putchard as *const ());
  }
}

pub trait JITter : ModuleProvider {
  // TODO: fix https://github.com/rust-lang/rust/issues/5665
  fn get_module_provider(&mut self) -> &mut ModuleProvider;
  fn run_function(&mut self, f: LLVMValueRef) -> f64;
}

struct ModulesContainer {
  execution_engines: Vec<ExecutionEngine>,
  modules: Vec<FrozenModule>
}

impl ModulesContainer {
  fn get_function_address(&self, name: &str) -> u64 {
    for ee in &self.execution_engines {
      let addr = ee.get_function_address(name);
      if addr != 0 {
        return addr;
      }
    }
    0
  }
}

pub struct MCJITter {
  module_name: String,
  current_module: core::Module,
  func_pass_manager: core::FunctionPassManager,
  container: Rc<RefCell<ModulesContainer>>
}

impl MCJITter {
  pub fn new(name: &str) -> MCJITter {
    let (current_module, func_pass_manager) = module::new_module(name);

    MCJITter {
      module_name: name.to_string(),
      current_module: current_module,
      func_pass_manager: func_pass_manager,
      container: Rc::new(RefCell::new(ModulesContainer {
        execution_engines: vec![],
        modules: vec![]
      }))
    }
  }

  fn close_current_module(&mut self) {
    let (new_module, new_func_pass_manager) = module::new_module(&self.module_name);
    self.func_pass_manager = new_func_pass_manager;
    let current_module = std::mem::replace(&mut self.current_module, new_module);

    let container = self.container.clone();
    let memory_manager = BindingSectionMemoryManagerBuilder::new()
      // symbol resolution
      .set_get_symbol_address(move |mut parent_mm, name| {
        let addr = parent_mm.get_symbol_address(name);
        if addr != 0 {
          return addr;
        }

        container.borrow().get_function_address(name)
      })
      .create();

      let (execution_engine, module) = match MCJITBuilder::new()
        .set_mcjit_memory_manager(Box::new(memory_manager))
        .create(current_module) {
          Ok((ee, module)) => (ee, module),
          Err(msg) => panic!(msg)
      };

      self.container.borrow_mut().execution_engines.push(execution_engine);
      self.container.borrow_mut().modules.push(module);
  }
}

impl ModuleProvider for MCJITter {
  fn dump(&self) {
    for module in self.container.borrow().modules.iter() {
      module.get().dump();
    }
    self.current_module.dump();
  }

  fn get_module(&mut self) -> &mut core::Module {
    &mut self.current_module
  }

  fn get_pass_manager(&mut self) -> &mut core::FunctionPassManager {
    &mut self.func_pass_manager
  }

  fn get_function(&mut self, name: &str) -> Option<(FunctionRef, bool)> {
    for module in &self.container.borrow().modules {
      let funct = match module.get().get_function_by_name(name) {
        Some(f) => {
          // found previously defined function
          f
        },
        None => continue
      };

      let proto = match self.current_module.get_function_by_name(name) {
        Some(f) => {
          // function redefinition
          if funct.count_basic_blocks() != 0 && f.count_basic_blocks() != 0 {
            panic!("function redefinition across modules")
          }
          f
        },
        None => {
          // function of the current module is the prototype
          let fty = unsafe { FunctionTypeRef::from_ref(funct.get_type().to_ref()) };
          let fty = unsafe { FunctionTypeRef::from_ref(fty.get_return_type().to_ref()) };
          FunctionRef::new(&mut self.current_module, name, &fty)
        }
      };

      if funct.count_basic_blocks() > 0 {
        // Found previously defined function for the prototype of the current module
        return Some((proto, true))
      }
    }

    match self.current_module.get_function_by_name(name) {
      Some(f) => Some((f, f.count_basic_blocks() > 0)),
      None => None
    }
  }
}

impl JITter for MCJITter {
  fn get_module_provider(&mut self) -> &mut ModuleProvider {
    self
  }

  fn run_function(&mut self, f: LLVMValueRef) -> f64 {
    self.close_current_module();
    let f = unsafe {FunctionRef::from_ref(f)};
    let mut args = vec![];
    let res = self.container.borrow().execution_engines.last().expect("MCJITter went craze")
      .run_function(&f, args.as_mut_slice());
    let ty = RealTypeRef::get_double();
    res.to_float(&ty)
  }
}
