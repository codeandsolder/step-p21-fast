use super::{
    Constraints, Legalize, SemanticError,
    entity::Entity,
    namespace::Namespace,
    scope::{Scope, ScopeType},
    type_decl::TypeDecl,
};
use crate::ast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub entities: Vec<Entity>,
    pub types: Vec<TypeDecl>,
}

impl Legalize for Schema {
    type Input = ast::Schema;

    fn legalize(
        ns: &Namespace,
        ss: &Constraints,
        scope: &Scope,
        schema: &Self::Input,
    ) -> Result<Self, SemanticError> {
        let name = schema.name.clone();
        let here = scope.pushed(ScopeType::Schema, &name);
        let entities = schema
            .entities
            .iter()
            .map(|entity| Entity::legalize(ns, ss, &here, entity))
            .collect::<Result<Vec<Entity>, _>>()?;
        let types = schema
            .types
            .iter()
            .map(|entity| TypeDecl::legalize(ns, ss, &here, entity))
            .collect::<Result<Vec<TypeDecl>, _>>()?;
        Ok(Self {
            name,
            entities,
            types,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SyntaxTree;

    #[test]
    fn legalize() {
        let example = SyntaxTree::example();
        let ns = Namespace::new(&example);
        let ss = Constraints::new(&ns, &example).unwrap();
        let schema = &example.schemas[0];
        let scope = Scope::root();
        let _schema = Schema::legalize(&ns, &ss, &scope, schema).unwrap();
    }
}
