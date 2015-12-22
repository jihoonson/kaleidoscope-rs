use std::collections::HashMap;
use llvm_sys::prelude::LLVMValueRef;

use iron_llvm::core;
use iron_llvm::core::types::{RealTypeCtor, RealTypeRef};
use iron_llvm::{LLVMRef, LLVMRefCtor};

pub struct Context {
  context: core::Context,
  builder: core::Builder,
  named_values: HashMap<String, LLVMValueRef>,
  ty: RealTypeRef,
}

impl Context {
  pub fn new() -> Context {
    Context {
      context: core::Context::get_global(),
      builder: core::Builder::new(),
      named_values: HashMap::new(),
      ty: RealTypeRef::get_double()
    }
  }
}
