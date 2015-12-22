extern crate llvm_sys;
extern crate iron_llvm;

use core::LLVMValueRef;
use parser;

pub type IRBuildingResult = Result<(LLVMValueRef, bool), String>;

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
      &parser::ExternNode(ref prototype) => prototpye.codegen(context, module_provider),
      &parser::FunctionNode(ref function) => function.codegen(context, module_provider)
    }
  }
}

impl IRBuilder for parser::Prototype {
  fn codegen(&self, context: &mut Context, module_provider: &mut ModuleProvider) -> IRBuildingResult {
    let function = match module_provider.get_function(&self.name) {
      Some((prev_def, redef)) => {
        if (prev_def.count_params() as usize != self.args.len()) {
          return error("redefinition of function with different number of args")
        }

        if (redef) {
          return error("redefinition of function")
        }

        prev_def
      },
      None => {
        let mut param_types = iter::repeat(context.ty.to_ref()).take(self.args.len()).collect::<Vec<_>>();
        let fty = FunctionTypeRef::get(&context.ty, param_types.as_mut_slice(), false);
        FunctionTypeRef::new(&mut module_provider.get_module(), &self.name, &fty)
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

    let body = match self.body.codegen(context, module_provider) {
      Ok((value, _)) => value,
      Err(message) => {
        unsafe { LLVMDeleteFunction(function.to_ref()) };
        return Err(message);
      }
    };

    context.builder.build_ret(&body);

    function.verify(LLVMAbortProcessAction);

    context.named_values.clear();
    Ok((function.to_ref(), self.prototype.name.as_str() == ""))
  }
}
