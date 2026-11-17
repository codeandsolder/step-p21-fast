//! AST for function, procedure, and rule declarations

use crate::{
    ast::{Entity, Expression, Qualifier, SubTypeConstraint, Type, TypeDecl},
    derive_ast_component,
    parser::{function_decl, procedure_decl},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Alias {
        name: String,
        dest: String,
        qualifiers: Vec<Qualifier>,
        statements: Vec<Self>,
    },

    Assignment {
        name: String,
        qualifiers: Vec<Qualifier>,
        expr: Expression,
    },

    Compound {
        statements: Vec<Self>,
    },

    If {
        condition: Expression,
        then_branch: Vec<Self>,
        else_branch: Option<Vec<Self>>,
    },

    Case {
        selector: Expression,
        actions: Vec<(Vec<Expression>, Self)>,
        otherwise: Option<Box<Self>>,
    },

    Repeat {
        /// Boxed: `RepeatControl` is by far the largest thing a `Statement`
        /// can hold, and leaving it inline made EVERY statement ~408
        /// bytes against 120 for the next-largest variant.
        control: Box<RepeatControl>,
        statements: Vec<Self>,
    },

    Return {
        value: Option<Expression>,
    },

    ProcedureCall {
        procedure: ProcedureCallName,
        parameters: Option<Vec<Expression>>,
    },

    Skip,
    Escape,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatControl {
    pub increment: Option<RepeatIncrement>,
    pub while_: Option<Expression>,
    pub until: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatIncrement {
    pub variable: String,
    pub begin: Expression,
    pub end: Expression,
    pub increment: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    pub name: String,
    pub parameters: Vec<FormalParameter>,
    pub declarations: Vec<Declaration>,
    pub constants: Vec<Constant>,
    pub variables: Vec<LocalVariable>,
    pub statements: Vec<Statement>,
}

derive_ast_component!(Procedure, procedure_decl);

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<FormalParameter>,
    pub declarations: Vec<Declaration>,
    pub constants: Vec<Constant>,
    pub variables: Vec<LocalVariable>,
    pub statements: Vec<Statement>,
    pub return_type: Type,
}

derive_ast_component!(Function, function_decl);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcedureCallName {
    Reference(String),
    /// Built-in procedure `INSERT`
    Insert,
    /// Built-in procedure `REMOVE`
    Remove,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormalParameter {
    pub name: String,
    pub ty: Type,
    /// `true` if specified with `VAR` in `PROCEDURE`. Always `false` for
    /// `FUNCTION`
    pub is_variable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constant {
    pub name: String,
    pub ty: Type,
    pub expr: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub name: String,
    pub references: Vec<String>,
    pub declarations: Vec<Declaration>,
    pub constants: Vec<Constant>,
    pub variables: Vec<LocalVariable>,
    pub statements: Vec<Statement>,
    pub where_clause: WhereClause,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalVariable {
    pub name: String,
    pub ty: Type,
    pub expr: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceSpec {
    Reference {
        name: String,
        resources: Vec<(String, Option<String>)>,
    },
    Use {
        name: String,
        types: Vec<(String, Option<String>)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub rules: Vec<DomainRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainRule {
    pub label: Option<String>,
    pub expr: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Entity(Entity),
    Type(TypeDecl),
    Function(Function),
    Procedure(Procedure),
    Rule(Rule),
    SubTypeConstraint(SubTypeConstraint),
}
