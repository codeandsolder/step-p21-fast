//! AST for expression

use derive_more::From;

#[cfg(doc)]
use crate::parser::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Unary {
        op: UnaryOperator,
        arg: Box<Self>,
    },
    Binary {
        op: BinaryOperator,
        arg1: Box<Self>,
        arg2: Box<Self>,
    },
    Relation {
        op: RelationOperator,
        lhs: Box<Self>,
        rhs: Box<Self>,
    },
    Literal(Literal),
    QualifiableFactor {
        factor: QualifiableFactor,
        qualifiers: Vec<Qualifier>,
    },
    EntityConstructor {
        name: String,
        values: Vec<Self>,
    },
    Interval {
        op_low: IntervalOperator,
        op_high: IntervalOperator,
        high: Box<Self>,
        low: Box<Self>,
        item: Box<Self>,
    },
    EnumerationReference {
        ty: Option<String>,
        enum_ref: String,
    },
    AggregateInitializer {
        elements: Vec<Element>,
    },
    Query {
        variable: String,
        source: Box<Self>,
        expr: Box<Self>,
    },
}

macro_rules! impl_relation_op_expression {
    ($f:ident, $op:path) => {
        #[must_use]
        pub fn $f(self, other: Self) -> Self {
            Expression::Relation {
                op: $op,
                lhs: Box::new(self),
                rhs: Box::new(other),
            }
        }
    };
}

macro_rules! impl_binary_op_expression {
    ($f:ident, $op:path) => {
        #[must_use]
        pub fn $f(self, other: Self) -> Self {
            Expression::Binary {
                op: $op,
                arg1: Box::new(self),
                arg2: Box::new(other),
            }
        }
    };
}

impl Expression {
    impl_relation_op_expression!(leq, RelationOperator::Leq);

    impl_relation_op_expression!(geq, RelationOperator::Geq);

    impl_relation_op_expression!(lt, RelationOperator::Lt);

    impl_relation_op_expression!(gt, RelationOperator::Gt);

    impl_relation_op_expression!(eq, RelationOperator::Equal);

    impl_relation_op_expression!(neq, RelationOperator::NotEqual);

    impl_relation_op_expression!(in_, RelationOperator::In);

    impl_relation_op_expression!(like, RelationOperator::Like);

    impl_binary_op_expression!(and, BinaryOperator::And);

    impl_binary_op_expression!(or, BinaryOperator::Or);

    impl_binary_op_expression!(xor, BinaryOperator::Xor);

    /// `SELF` constant
    #[must_use]
    pub fn self_() -> Self {
        Self::self_qualified(Vec::new())
    }

    /// `SELF` constant with qualifiers
    #[must_use]
    pub const fn self_qualified(qualifiers: Vec<Qualifier>) -> Self {
        Self::QualifiableFactor {
            factor: QualifiableFactor::BuiltInConstant(BuiltInConstant::Self_),
            qualifiers,
        }
    }

    /// `?` constant
    #[must_use]
    pub const fn indeterminate() -> Self {
        Self::QualifiableFactor {
            factor: QualifiableFactor::BuiltInConstant(
                BuiltInConstant::Indeterminate,
            ),
            qualifiers: Vec::new(),
        }
    }

    /// Real value literal
    #[must_use]
    pub const fn real(value: f64) -> Self {
        Self::Literal(Literal::Real(value))
    }
}

macro_rules! impl_binary_op_expression {
    ($f:ident, $ops:path, $op:path) => {
        impl $ops for Expression {
            type Output = Self;

            fn $f(self, other: Expression) -> Self {
                Expression::Binary {
                    op: $op,
                    arg1: Box::new(self),
                    arg2: Box::new(other),
                }
            }
        }
    };
}

impl_binary_op_expression!(add, std::ops::Add, BinaryOperator::Add);
impl_binary_op_expression!(sub, std::ops::Sub, BinaryOperator::Sub);
impl_binary_op_expression!(mul, std::ops::Mul, BinaryOperator::Mul);

#[derive(Debug, Clone, PartialEq)]
pub enum QualifiableFactor {
    /// [`attribute_ref`], [`general_ref`], [population], or [`constant_ref`]
    Reference(String),
    /// [`built_in_constant`]
    BuiltInConstant(BuiltInConstant),
    /// [`function_call`]
    FunctionCall {
        name: FunctionCallName,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionCallName {
    BuiltInFunction(BuiltInFunction),
    Reference(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltInFunction {
    Abs,
    Acos,
    Asin,
    Atan,
    Blength,
    Cos,
    Exists,
    Exp,
    Format,
    Hibound,
    Hiindex,
    Length,
    Lobound,
    Loindex,
    Log,
    Log2,
    Log10,
    Nvl,
    Odd,
    Rolesof,
    Sin,
    Sizeof,
    Sqrt,
    Tan,
    Typeof,
    Usedin,
    Value,
    ValueIn,
    ValueUnique,
}

/// Output of [qualifier]
#[derive(Debug, Clone, PartialEq)]
pub enum Qualifier {
    /// Like `.x`
    Attribute(String),
    /// Like `\point`
    Group(String),
    /// Like `[1]`
    Index(Expression),
    /// Like `[1:3]`
    Range { begin: Expression, end: Expression },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInConstant {
    /// `CONST_E`, Napier's constant `e = 2.71828 …`
    Napier,
    /// The ratio of a circle's circumference to its diameter, `π = 3.14159 …`
    Pi,
    /// `SELF` is not a constant, but behaves as one in every context in which
    /// it can appear.
    Self_,
    /// The indeterminate symbol `?` stands for an ambiguous value.
    /// It is compatible with all data types.
    Indeterminate,
}

/// Relation operators parsed by [`rel_op`] and [`rel_op_extended`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationOperator {
    /// `=`
    Equal,
    /// `<>`
    NotEqual,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Leq,
    /// `>=`
    Geq,
    /// `:=:`
    InstanceEqual,
    /// `:<>:`
    InstanceNotEqual,

    /* extended */
    /// `IN`
    In,
    /// `LIKE`
    Like,
}

/// Unary operators parsed by [`unary_op`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `NOT`
    Not,
}

/// Binary operators parsed by [`add_like_op`], [`multiplication_like_op`], and
/// [`power_op`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /* Mul-like */
    /// `*`
    Mul,
    /// `/`
    RealDiv,
    /// `DIV`
    IntegerDiv,
    /// `MOD`
    Mod,
    /// `AND`
    And,
    /// `||`, Complex entity instance construction operator (12.10)
    ComplexEntityInstanceConstruction,

    /* Add-like */
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `OR`
    Or,
    /// `XOR`
    Xor,

    /* power */
    /// `**`
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalOperator {
    /// `<`
    LessThan,
    /// `<=`
    LessThanEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub expr: Expression,
    pub repetition: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Logical {
    False,
    True,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, From)]
pub enum Literal {
    Real(f64),
    String(String),
    Logial(Logical),
}
