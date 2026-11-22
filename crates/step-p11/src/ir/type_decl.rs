use super::{
    Constraints, Legalize, Namespace, Scope, SemanticError, SimpleType, TypeRef,
};
use crate::ast;

/// Rename of primitive type,
/// e.g. `TYPE label = STRING; ENDTYPE;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simple {
    pub id: String,
    pub ty: SimpleType,
}

/// Rename of user defined type,
/// e.g. `TYPE box_height = positive_ratio_measure; END_TYPE;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub id: String,
    pub ty: TypeRef,
}

/// Enumeration of values,
/// e.g. `TYPE text_path = ENUMERATION OF (up, right, down, left); END_TYPE;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enumeration {
    pub id: String,
    pub items: Vec<String>,
}

/// Select of user defined types,
/// e.g. `TYPE geometric_set_select = SELECT (point, curve); END_TYPE;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select {
    pub id: String,
    pub types: Vec<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDecl {
    Simple(Simple),
    Rename(Rename),
    Enumeration(Enumeration),
    Select(Select),
}

impl TypeDecl {
    #[must_use = "the returned identifier names the type declaration"]
    pub fn id(&self) -> &str {
        match self {
            Self::Simple(e) => &e.id,
            Self::Rename(e) => &e.id,
            Self::Enumeration(e) => &e.id,
            Self::Select(e) => &e.id,
        }
    }
}

impl Legalize for TypeDecl {
    type Input = ast::TypeDecl;

    fn legalize(
        ns: &Namespace,
        ss: &Constraints,
        scope: &Scope,
        type_decl: &Self::Input,
    ) -> Result<Self, SemanticError> {
        use ast::Type;
        let id = type_decl.type_id.clone();
        Ok(match &type_decl.underlying_type {
            Type::Simple(ty) => Self::Simple(Simple {
                id,
                ty: SimpleType(*ty),
            }),
            Type::Named(name) => {
                let (path, _index) = ns.resolve(scope, name)?;
                Self::Rename(Rename {
                    id,
                    ty: TypeRef::from_path(ns, ss, &path)?,
                })
            }
            Type::Enumeration {
                items,
                extensibility: _,
            } => Self::Enumeration(Enumeration {
                id,
                items: items.clone(),
            }),
            Type::Select {
                types,
                extensibility: _,
            } => {
                let types = types
                    .iter()
                    .map(|ty| {
                        let (path, _index) = ns.resolve(scope, ty)?;
                        TypeRef::from_path(ns, ss, &path)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Select(Select { id, types })
            }
            Type::Set { base, bound } => {
                let base = TypeRef::legalize(ns, ss, scope, base.as_ref())?;
                let bound = if let Some(bound) = bound {
                    Some(Legalize::legalize(ns, ss, scope, bound)?)
                } else {
                    None
                };
                Self::Rename(Rename {
                    id,
                    ty: TypeRef::Set {
                        base: Box::new(base),
                        bound,
                    },
                })
            }
            Type::List {
                base,
                bound,
                unique,
            } => {
                let base = TypeRef::legalize(ns, ss, scope, base.as_ref())?;
                let bound = if let Some(bound) = bound {
                    Some(Legalize::legalize(ns, ss, scope, bound)?)
                } else {
                    None
                };
                Self::Rename(Rename {
                    id,
                    ty: TypeRef::List {
                        base: Box::new(base),
                        bound,
                        unique: *unique,
                    },
                })
            }
            unsupported => {
                return Err(SemanticError::UnsupportedConstruct {
                    context: "type declaration",
                    detail: format!("{unsupported:?}"),
                });
            }
        })
    }
}
