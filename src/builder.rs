use std::iter;
use parser;
use context::Context;
use module::ModuleProvider;

use llvm_sys::LLVMRealPredicate::LLVMRealOLT;
use llvm_sys::analysis::LLVMVerifierFailureAction::LLVMAbortProcessAction;
use llvm_sys::core::LLVMDeleteFunction;
use llvm_sys::prelude::LLVMValueRef;

use iron_llvm::{LLVMRef, LLVMRefCtor};
use iron_llvm::core;
use iron_llvm::core::value::*;
// use iron_llvm::core::value::{Function, FunctionCtor, FunctionRef, RealConstRef, RealConstCtor};
use iron_llvm::core::types::{FunctionTypeCtor, FunctionTypeRef, RealTypeCtor, RealTypeRef};

pub type Runnable = bool;
pub type IRBuildingResult = Result<(LLVMValueRef, Runnable), String>;

fn error(message : &str) -> IRBuildingResult {
  Err(message.to_string())
}

pub trait IRBuilder {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult;
}

impl IRBuilder for parser::ParsingResult {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    match self {
      &Ok((ref ast, _)) => ast.codegen(context, module_provider),
      &Err(ref message) => Err(message.clone())
    }
  }
}

impl IRBuilder for Vec<parser::ASTNode> {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    let mut result = error("empty AST");
    for node in self.iter() {
      result = Ok(try!(node.codegen(context, module_provider)));
    }
    result
  }
}

impl IRBuilder for parser::ASTNode {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    match self {
      &parser::ExternNode(ref prototype) => prototype.codegen(context, module_provider),
      &parser::FunctionNode(ref function) => function.codegen(context, module_provider)
    }
  }
}

impl IRBuilder for parser::Prototype {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    let function = match module_provider.get_function(&self.name) {
      Some((prev_def, redef)) => {
        if prev_def.count_params() as usize != self.args.len() {
          return error("redefinition of function with different number of args")
        }

        if redef {
          return error("redefinition of function")
        }

        prev_def
      },
      None => {
        let mut param_types = iter::repeat(context.ty.to_ref()).take(self.args.len()).collect::<Vec<_>>();
        let fty = FunctionTypeRef::get(&context.ty, param_types.as_mut_slice(), false);
        FunctionRef::new(&mut module_provider.get_module(), &self.name, &fty)
      }
    };

    for (param, arg) in function.params_iter().zip(&self.args) {
      param.set_name(arg);
    }

    Ok((function.to_ref(), false))
  }
}

impl IRBuilder for parser::Function {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    // Since there are no global variables, it's ok to remove all variables which are defined before.
    context.named_values.clear();

    let (function, _) = try!(self.prototype.codegen(context, module_provider));
    let mut function = unsafe { FunctionRef::from_ref(function) };

    let mut bb = function.append_basic_block_in_context(&mut context.context, "entry");
    context.builder.position_at_end(&mut bb);

    for (param, arg) in function.params_iter().zip(&self.prototype.args) {
      context.named_values.insert(arg.clone(), param.to_ref());
    }

    let body = match self.expression.codegen(context, module_provider) {
      Ok((value, _)) => value,
      Err(message) => {
        unsafe { LLVMDeleteFunction(function.to_ref()) };
        return Err(message);
      }
    };

    context.builder.build_ret(&body);

    function.verify(LLVMAbortProcessAction);
    module_provider.get_pass_manager().run(&mut function);

    context.named_values.clear();
    Ok((function.to_ref(), self.prototype.name.as_str() == ""))
  }
}

impl IRBuilder for parser::Expression {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    match self {
      &parser::LiteralExpr(ref value) => {
        Ok((RealConstRef::get(&context.ty, *value).to_ref(), false))
      },

      &parser::VariableExpr(ref name) => {
        match context.named_values.get(name) {
          Some(value) => {
            Ok((*value, false))
          },
          None => error("unknown variable name")
        }
      },

      &parser::BinaryExpr(ref name, ref lhs, ref rhs) => {
        let (lhs_value, _) = try!(lhs.codegen(context, module_provider));
        let (rhs_value, _) = try!(rhs.codegen(context, module_provider));

        match name.as_str() {
          "+" => Ok((context.builder.build_fadd(lhs_value, rhs_value, "addtmp"), false)),
          "-" => Ok((context.builder.build_fsub(lhs_value, rhs_value, "subtmp"), false)),
          "*" => Ok((context.builder.build_fmul(lhs_value, rhs_value, "multmp"), false)),
          "<" => {
            let cmp = context.builder.build_fcmp(LLVMRealOLT, lhs_value, rhs_value, "cmptmp");

            // convert boolean to double 0.0 or 1.0
            Ok((context.builder.build_ui_to_fp(cmp, context.ty.to_ref(), "booltmp"), false))
          },
          _ => error("invalid binary operator")
        }
      },

      &parser::CallExpr(ref name, ref args) => {
        let (function, _) = match module_provider.get_function(name) {
          Some(function) => function,
          None => return error("unknown functino referenced")
        };

        if function.count_params() as usize != args.len() {
          return error("incorrect number of arguments passed")
        }

        let mut args_value = Vec::new();
        for arg in args.iter() {
          let (arg_value, _) = try!(arg.codegen(context, module_provider));
          args_value.push(arg_value);
        }

        Ok((context.builder.build_call(function.to_ref(), args_value.as_mut_slice(), "calltmp"), false))
      }
    }
  }
}
