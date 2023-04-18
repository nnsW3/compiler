mod block;
mod builder;
mod dataflow;
mod function;
mod immediates;
mod instruction;
mod layout;
mod value;
mod write;

pub use self::block::{Block, BlockData};
pub use self::builder::{InstBuilder, InstBuilderBase};
pub use self::dataflow::DataFlowGraph;
pub use self::function::{FuncRef, Function, Signature, Visibility};
pub use self::immediates::Immediate;
pub use self::instruction::*;
pub use self::layout::{ArenaMap, LayoutAdapter, LayoutNode, OrderedArenaMap};
pub use self::value::{Value, ValueData, ValueList, ValueListPool};
pub use self::write::write_function;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use cranelift_entity::PrimaryMap;

use miden_diagnostics::SourceSpan;

/// Represents a SSA IR module
///
/// This module is largely a container for functions, but it also acts
/// as the owner for pooled resources available to functions:
///
/// * Mapping from Signature to FuncRef
/// * Mapping from FunctionName to FuncRef
#[derive(Debug)]
pub struct Module {
    /// The name of this module
    pub name: String,
    /// The source span from which this module was parsed, if available
    pub span: SourceSpan,
    /// This is the list of functions defined in this module
    pub functions: Vec<Function>,
    /// This map associates function references to metadata about that function (arity, type, visibility, etc.)
    ///
    /// NOTE: The functions referenced here are not necessarily defined in this module
    pub signatures: Rc<RefCell<PrimaryMap<FuncRef, Signature>>>,
    /// This map provides the ability to look up function references by name.
    ///
    /// Every entry in this table corresponds to an entry in `signatures`
    pub names: Rc<RefCell<BTreeMap<String, FuncRef>>>,
}
impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "module {}\n", &self.name)?;
        for function in self.functions.iter() {
            writeln!(f)?;
            write_function(f, function)?;
        }

        Ok(())
    }
}
impl Module {
    pub fn new(name: String, span: Option<SourceSpan>) -> Self {
        Self {
            name,
            span: span.unwrap_or_default(),
            functions: vec![],
            signatures: Rc::new(RefCell::new(PrimaryMap::new())),
            names: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    /// If the given function is defined in this module, return true
    pub fn is_local(&self, name: &str) -> bool {
        if let Some(id) = self.get_funcref_by_name(name) {
            let sigs = self.signatures.borrow();
            sigs.get(id)
                .map(|sig| !sig.visibility.is_externally_defined())
                .is_some()
        } else {
            false
        }
    }

    pub fn get_funcref_by_name(&self, name: &str) -> Option<FuncRef> {
        self.names.borrow().get(name).copied()
    }

    /// Returns a reference to the definition of the given FuncRef, if it refers to a local definition
    pub fn get_function(&self, id: FuncRef) -> Option<&Function> {
        self.functions.iter().find(|f| f.id == id)
    }

    /// Returns a mutable reference to the definition of the given FuncRef, if it refers to a local definition
    pub fn get_function_mut(&mut self, id: FuncRef) -> Option<&mut Function> {
        self.functions.iter_mut().find(|f| f.id == id)
    }

    /// Declares a function in the current module with the given signature, and creates the empty
    /// definition for it. Use the returned `FuncRef` to obtain a reference to that definition using
    /// `get_function` or `get_function_mut`.
    pub fn declare_function(&mut self, signature: Signature) -> FuncRef {
        let mut signatures = self.signatures.borrow_mut();
        let mut names = self.names.borrow_mut();
        // Register the signature
        let name = signature.name.clone();
        let f = signatures.push(signature);
        // Register both the fully-qualified and local names
        names.insert(name, f);
        f
    }

    /// Adds the definition of a function which was previously declared
    ///
    /// This function will panic if the function provided is not declared in this module
    pub fn define_function(&mut self, function: Function) {
        let signatures = self.signatures.borrow();
        assert!(
            signatures.get(function.id).is_some(),
            "cannot define a function which was not declared in this module"
        );

        self.functions.push(function);
    }
}
