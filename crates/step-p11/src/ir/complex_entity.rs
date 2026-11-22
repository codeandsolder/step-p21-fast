//! Partial complex entities described in ISO-10303-11 Annex B
use super::{ConstraintExpr, Namespace, Path, SemanticError};

use std::cmp::Ordering;

/// Partial complex entity data type, e.g. $A \And B \And C$ in ISO document
///
/// Each component, e.g. $A$, will be represented by an index.
/// $\And$ operation is implemented by [`std::ops::BitAnd`] trait.
/// This satisfies following equations:
///
/// - $A \And A = A$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// assert_eq!(a.clone() & a.clone(), a);
/// ```
///
/// - $A \And B = B \And A$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b = PartialComplexEntity::new(&[2]);
/// assert_eq!(a.clone() & b.clone(), b & a);
/// ```
///
/// - $A \And (B \And C) = (A \And B) \And C = A \And B \And C$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b = PartialComplexEntity::new(&[3]);
/// let c = PartialComplexEntity::new(&[2]);
/// assert_eq!(
///     (a.clone() & b.clone()) & c.clone(),
///     a.clone() & (b.clone() & c.clone())
/// );
/// assert_eq!(a & b & c, PartialComplexEntity::new(&[1, 2, 3]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialComplexEntity {
    /// Sorted and non-duplicated indices
    indices: Vec<usize>,
}

impl PartialOrd for PartialComplexEntity {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for PartialComplexEntity {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match Ord::cmp(&self.indices.len(), &rhs.indices.len()) {
            Ordering::Equal => Ord::cmp(&self.indices, &rhs.indices),
            a @ (Ordering::Less | Ordering::Greater) => a,
        }
    }
}

impl PartialComplexEntity {
    #[must_use = "the constructed partial entity must be used"]
    pub fn new(indices: &[usize]) -> Self {
        let mut indices = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        Self { indices }
    }

    /// Restore paths from namespace indices.
    ///
    /// # Errors
    /// Returns an error if an index does not belong to the namespace.
    pub fn as_path(&self, ns: &Namespace) -> Result<Vec<Path>, SemanticError> {
        self.indices
            .iter()
            .map(|&index| {
                ns.ast
                    .get(index)
                    .map(|(path, _ast)| path.clone())
                    .ok_or(SemanticError::InvalidNamespaceIndex { index })
            })
            .collect()
    }
}

impl From<&[usize]> for PartialComplexEntity {
    fn from(indices: &[usize]) -> Self {
        Self::new(indices)
    }
}

impl std::ops::BitAnd for PartialComplexEntity {
    type Output = Self;

    fn bitand(mut self, mut rhs: Self) -> Self {
        self.indices.append(&mut rhs.indices);
        self.indices.sort_unstable();
        self.indices.dedup();
        self
    }
}

/// Instantiable subtypes described by a list of partial complex entity, e.g.
/// $[A, B & C]$
///
/// This has several operation described in ISO-10303-11 annex B,
/// $+$, $-$, $\And$, and $/$.
///
/// - $A + [`B_1`, `B_2`] = [`B_1`, `B_2`] + A = [A, `B_1`, `B_2`]$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b1 = PartialComplexEntity::new(&[2]);
/// let b2 = PartialComplexEntity::new(&[3]);
///
/// let ce = Instantiables::new(&[b1.clone(), b2.clone()]);
///
/// assert_eq!(
///     a.clone() + ce,
///     Instantiables::new(&[a.clone(), b1.clone(), b2.clone(),])
/// );
/// ```
///
/// - $A \And [`B_1`, `B_2`] = [`B_1`, `B_2`] \And A = [A \And `B_1`, A \And
///   `B_2`]$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b1 = PartialComplexEntity::new(&[2]);
/// let b2 = PartialComplexEntity::new(&[3]);
///
/// let ce = Instantiables::new(&[b1.clone(), b2.clone()]);
///
/// assert_eq!(
///     a.clone() & ce,
///     Instantiables::new(&[a.clone() & b1.clone(), a.clone() & b2.clone(),])
/// );
/// ```
///
/// - $[`A_1`, `A_2`] + [`B_1`, `B_2`] = [`A_1`, `A_2`, `B_1`, `B_2`]$
///
/// ```
/// # use step_p11::ir::*;
/// let a1 = PartialComplexEntity::new(&[1]);
/// let a2 = PartialComplexEntity::new(&[2]);
/// let b1 = PartialComplexEntity::new(&[3]);
/// let b2 = PartialComplexEntity::new(&[4]);
///
/// let ce1 = Instantiables::new(&[a1.clone(), a2.clone()]);
/// let ce2 = Instantiables::new(&[b1.clone(), b2.clone()]);
///
/// assert_eq!(
///     ce1 + ce2,
///     Instantiables::new(&[a1.clone(), a2.clone(), b1.clone(), b2.clone(),])
/// );
/// ```
///
/// - $[`A_1`, `A_2`] \And [`B_1`, `B_2`] = [`A_1` \And `B_1`, `A_1` \And `B_2`,
///   `A_2` \And `B_1`, `A_2` \And `B_2`]$
///
/// ```
/// # use step_p11::ir::*;
/// let a1 = PartialComplexEntity::new(&[1]);
/// let a2 = PartialComplexEntity::new(&[2]);
/// let b1 = PartialComplexEntity::new(&[3]);
/// let b2 = PartialComplexEntity::new(&[4]);
///
/// let ce1 = Instantiables::new(&[a1.clone(), a2.clone()]);
/// let ce2 = Instantiables::new(&[b1.clone(), b2.clone()]);
///
/// assert_eq!(
///     ce1 & ce2,
///     Instantiables::new(&[
///         a1.clone() & b1.clone(),
///         a1.clone() & b2.clone(),
///         a2.clone() & b1.clone(),
///         a2.clone() & b2.clone(),
///     ])
/// );
/// ```
///
/// - $[A, A \And B, A \And C, A \And B \And D, B \And C, D] / A = [A, A \And B,
///   A \And C, A \And B \And D]$
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b = PartialComplexEntity::new(&[2]);
/// let c = PartialComplexEntity::new(&[3]);
/// let d = PartialComplexEntity::new(&[4]);
///
/// let ce = Instantiables::new(&[
///     a.clone(),
///     a.clone() & b.clone(),
///     a.clone() & c.clone(),
///     a.clone() & b.clone() & d.clone(),
///     b.clone() & c.clone(),
///     d.clone(),
/// ]);
///
/// assert_eq!(
///     ce / a.clone(),
///     Instantiables::new(&[
///         a.clone(),
///         a.clone() & b.clone(),
///         a.clone() & c.clone(),
///         a.clone() & b.clone() & d.clone(),
///     ])
/// );
/// ```
///
/// - $ [A, A \And B, A \And C, A \And B \And D, B \And C, D]/[B, D] = [A \And
///   B, A \And B \And D, B \And C, D] $
///
/// ```
/// # use step_p11::ir::*;
/// let a = PartialComplexEntity::new(&[1]);
/// let b = PartialComplexEntity::new(&[2]);
/// let c = PartialComplexEntity::new(&[3]);
/// let d = PartialComplexEntity::new(&[4]);
///
/// let ce1 = Instantiables::new(&[
///     a.clone(),
///     a.clone() & b.clone(),
///     a.clone() & c.clone(),
///     a.clone() & b.clone() & d.clone(),
///     b.clone() & c.clone(),
///     d.clone(),
/// ]);
///
/// let ce2 = Instantiables::new(&[b.clone(), d.clone()]);
///
/// assert_eq!(
///     ce1 / ce2,
///     Instantiables::new(&[
///         a.clone() & b.clone(),
///         a.clone() & b.clone() & d.clone(),
///         b.clone() & c.clone(),
///         d.clone()
///     ])
/// );
/// ```
///
/// - $[`A_1`, `A_2`, `B_1`, `B_2`] − [`A_2`, `B_1`] = [`A_1`, `B_2`]$
///
/// ```
/// # use step_p11::ir::*;
/// let a1 = PartialComplexEntity::new(&[1]);
/// let a2 = PartialComplexEntity::new(&[2]);
/// let b1 = PartialComplexEntity::new(&[3]);
/// let b2 = PartialComplexEntity::new(&[4]);
///
/// let ce1 =
///     Instantiables::new(&[a1.clone(), a2.clone(), b1.clone(), b2.clone()]);
///
/// let ce2 = Instantiables::new(&[a2.clone(), b1.clone()]);
///
/// assert_eq!(ce1 - ce2, Instantiables::new(&[a1.clone(), b2.clone()]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Instantiables {
    /// Sorted and non-duplicated list of partial complex entities
    parts: Vec<PartialComplexEntity>,
}

impl Instantiables {
    #[must_use = "the constructed instantiable set must be used"]
    pub fn new(pces: &[PartialComplexEntity]) -> Self {
        Self::from_parts(pces.to_vec())
    }

    fn from_parts(mut parts: Vec<PartialComplexEntity>) -> Self {
        parts.sort_unstable();
        parts.dedup();
        Self { parts }
    }

    /// Create from single index
    #[must_use = "the constructed singleton instantiable set must be used"]
    pub fn single(index: usize) -> Self {
        Self {
            parts: vec![PartialComplexEntity::new(&[index])],
        }
    }

    /// ONEOF(A, B, C) -> [A, B, C]
    #[must_use = "the returned set contains the alternative combinations"]
    pub fn oneof(parts: Vec<Self>) -> Self {
        let mut is = Self::default();
        for p in parts {
            is = is + p;
        }
        is
    }

    /// A AND B AND C -> [A & B & C]
    #[must_use = "the returned set contains the conjunction combinations"]
    pub fn and(terms: Vec<Self>) -> Self {
        terms
            .into_iter()
            .reduce(std::ops::BitAnd::bitand)
            .unwrap_or_default()
    }

    /// A ANDOR B ANDOR C -> [A, B, C, A & B, B & C, A & C, A & B & C]
    #[must_use = "the returned set contains the nonempty factor combinations"]
    pub fn andor(factors: &[Self]) -> Self {
        // Maintain the set of all nonempty combinations incrementally.
        // This avoids bit-shift overflow when there are more factors than
        // machine-word bits and deduplicates as each factor is introduced.
        let mut combinations = Self::default();
        for factor in factors {
            let joint = combinations.clone() & factor.clone();
            combinations = combinations + factor.clone() + joint;
        }
        combinations
    }

    ///
    /// # Errors
    /// Returns a semantic analysis error if types, entities, or constraints
    /// cannot be resolved.
    pub fn from_constraint_expr(
        ns: &Namespace,
        expr: &ConstraintExpr,
    ) -> Result<Self, SemanticError> {
        use ConstraintExpr::{And, AndOr, OneOf, Reference};
        match expr {
            Reference(path) => {
                let (_ast, index) = ns.get(path)?;
                Ok(Self::single(index))
            }
            OneOf(exprs) => {
                let exprs = exprs
                    .iter()
                    .map(|e| Self::from_constraint_expr(ns, e))
                    .collect::<Result<Vec<Self>, SemanticError>>()?;
                Ok(Self::oneof(exprs))
            }
            And(exprs) => {
                let exprs = exprs
                    .iter()
                    .map(|e| Self::from_constraint_expr(ns, e))
                    .collect::<Result<Vec<Self>, SemanticError>>()?;
                Ok(Self::and(exprs))
            }
            AndOr(exprs) => {
                let exprs = exprs
                    .iter()
                    .map(|e| Self::from_constraint_expr(ns, e))
                    .collect::<Result<Vec<Self>, SemanticError>>()?;
                Ok(Self::andor(&exprs))
            }
        }
    }

    /// Restore paths from namespace indices.
    ///
    /// # Errors
    /// Returns an error if an index does not belong to the namespace.
    pub fn as_path(
        &self,
        ns: &Namespace,
    ) -> Result<Vec<Vec<Path>>, SemanticError> {
        self.parts.iter().map(|pce| pce.as_path(ns)).collect()
    }
}

impl<'a> FromIterator<&'a PartialComplexEntity> for Instantiables {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a PartialComplexEntity>,
    {
        Self::from_parts(iter.into_iter().cloned().collect())
    }
}

// [A, B] + [C, D] = [A, B, C, D]
impl std::ops::Add for Instantiables {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self {
        self.parts.append(&mut rhs.parts);
        Self::from_parts(self.parts)
    }
}

// [A, B] + C = [A, B, C]
impl std::ops::Add<PartialComplexEntity> for Instantiables {
    type Output = Self;

    fn add(mut self, rhs: PartialComplexEntity) -> Self {
        self.parts.push(rhs);
        Self::from_parts(self.parts)
    }
}

// A + [B, C] = [A, B, C]
impl std::ops::Add<Instantiables> for PartialComplexEntity {
    type Output = Instantiables;

    fn add(self, rhs: Instantiables) -> Instantiables {
        rhs + self
    }
}

// [A, B] & [C, D] = [A & C, A & D, B & C, B & D]
impl std::ops::BitAnd for Instantiables {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        let mut parts = Vec::with_capacity(self.parts.len() * rhs.parts.len());
        for p in &self.parts {
            for q in &rhs.parts {
                parts.push(p.clone() & q.clone());
            }
        }
        Self::from_parts(parts)
    }
}

// [A, B] & C = [A & C, B & C]
impl std::ops::BitAnd<PartialComplexEntity> for Instantiables {
    type Output = Self;

    fn bitand(self, q: PartialComplexEntity) -> Self {
        Self::from_parts(
            self.parts.into_iter().map(|p| p & q.clone()).collect(),
        )
    }
}

// A & [B, C] = [A & B, A & C]
impl std::ops::BitAnd<Instantiables> for PartialComplexEntity {
    type Output = Instantiables;

    fn bitand(self, rhs: Instantiables) -> Instantiables {
        rhs & self
    }
}

impl std::ops::Sub for Instantiables {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            parts: self
                .parts
                .into_iter()
                .filter(|p| rhs.parts.iter().all(|q| p != q))
                .collect(),
        }
    }
}

impl std::ops::Sub<PartialComplexEntity> for Instantiables {
    type Output = Self;

    fn sub(self, q: PartialComplexEntity) -> Self {
        Self {
            parts: self.parts.into_iter().filter(|p| p != &q).collect(),
        }
    }
}

// [A, A & B, A & C, A & B & D, B & C, D]/[B, D] ≡ [A & B, A & B & D, B & C, D]
impl std::ops::Div for Instantiables {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            parts: self
                .parts
                .into_iter()
                .filter(|p| {
                    for q in &rhs.parts {
                        if q.indices
                            .iter()
                            .all(|j| p.indices.binary_search(j).is_ok())
                        {
                            return true;
                        }
                    }
                    false
                })
                .collect(),
        }
    }
}

// [A, A & B, A & C, A & B & D, B & C, D] / A = [A, A & B, A & C, A & B & D]
impl std::ops::Div<PartialComplexEntity> for Instantiables {
    type Output = Self;

    fn div(self, rhs: PartialComplexEntity) -> Self {
        Self {
            parts: self
                .parts
                .into_iter()
                .filter(|part| {
                    rhs.indices
                        .iter()
                        .all(|i| part.indices.binary_search(i).is_ok())
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Instantiables, Namespace, PartialComplexEntity, SemanticError,
    };

    #[test]
    fn constructors_canonicalize_arbitrary_set_order_and_duplicates() {
        let left = PartialComplexEntity::new(&[3, 1, 3, 2, 1]);
        assert_eq!(left.indices, [1, 2, 3]);
        let right = PartialComplexEntity::new(&[2]);
        let set =
            Instantiables::new(&[left.clone(), right.clone(), left.clone()]);
        assert_eq!(set.parts, [right, left]);
    }

    #[test]
    fn resolving_an_out_of_range_index_returns_a_semantic_error() {
        let namespace = Namespace {
            names: Default::default(),
            ast: Vec::new(),
        };
        assert!(matches!(
            Instantiables::single(7).as_path(&namespace),
            Err(SemanticError::InvalidNamespaceIndex { index: 7 })
        ));
    }

    #[test]
    fn andor_covers_every_nonempty_factor_combination() {
        let a = Instantiables::single(1);
        let b = Instantiables::single(2);
        let c = Instantiables::single(3);
        let actual = Instantiables::andor(&[a.clone(), b.clone(), c.clone()]);
        let expected: &[&[usize]] =
            &[&[1], &[2], &[3], &[1, 2], &[1, 3], &[2, 3], &[1, 2, 3]];
        let actual_indices: Vec<_> = actual
            .parts
            .iter()
            .map(|part| part.indices.as_slice())
            .collect();
        assert_eq!(actual_indices, expected);
        assert!(Instantiables::andor(&[]).parts.is_empty());
        assert!(Instantiables::and(Vec::new()).parts.is_empty());
    }
}
