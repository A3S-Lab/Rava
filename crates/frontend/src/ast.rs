//! Java AST — full Java 21 syntax coverage.
//!
//! Covers: class/interface/enum declarations, method declarations, all statement
//! types (if, while, for, for-each, do-while, switch, try/catch/finally,
//! break, continue, synchronized, assert), all expression types (lambda,
//! method reference, ternary, cast, instanceof with pattern, array initializer).

/// A complete Java source file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub package:  Option<String>,
    pub imports:  Vec<String>,
    pub classes:  Vec<ClassDecl>,
}

/// A class, interface, or enum declaration.
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name:       String,
    pub kind:       ClassKind,
    pub modifiers:  Vec<Modifier>,
    pub superclass: Option<String>,
    pub interfaces: Vec<String>,
    pub members:    Vec<Member>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassKind {
    Class,
    Interface,
    Enum,
    Record,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Modifier {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Abstract,
    Synchronized,
    Native,
    Volatile,
    Transient,
    Strictfp,
    Default,
}

/// A class member.
#[derive(Debug, Clone)]
pub enum Member {
    Method(MethodDecl),
    Field(FieldDecl),
    Constructor(ConstructorDecl),
    StaticInit(Block),
    EnumConstant(EnumConstant),
    InnerClass(ClassDecl),
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name:       String,
    pub modifiers:  Vec<Modifier>,
    pub return_ty:  TypeExpr,
    pub params:     Vec<Param>,
    pub body:       Option<Block>,
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub name:      String,
    pub modifiers: Vec<Modifier>,
    pub params:    Vec<Param>,
    pub body:      Block,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name:      String,
    pub modifiers: Vec<Modifier>,
    pub ty:        TypeExpr,
    pub init:      Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct EnumConstant {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name:     String,
    pub ty:       TypeExpr,
    pub variadic: bool,
}

/// A type expression (e.g. `int`, `String`, `List<String>`, `int[]`).
#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub name:       String,
    pub array_dims: u8,
}

impl TypeExpr {
    pub fn simple(name: impl Into<String>) -> Self {
        Self { name: name.into(), array_dims: 0 }
    }
    pub fn array(name: impl Into<String>, dims: u8) -> Self {
        Self { name: name.into(), array_dims: dims }
    }
    pub fn is_void(&self) -> bool { self.name == "void" }
}

/// A block of statements.
#[derive(Debug, Clone)]
pub struct Block(pub Vec<Stmt>);

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `expr;`
    Expr(Expr),
    /// `return expr?;`
    Return(Option<Expr>),
    /// `Type name = init?;` or `var name = init;`
    LocalVar { ty: TypeExpr, name: String, init: Option<Expr> },
    /// `if (cond) then else?`
    If { cond: Expr, then: Box<Stmt>, else_: Option<Box<Stmt>> },
    /// `while (cond) body`
    While { cond: Expr, body: Box<Stmt> },
    /// `do body while (cond);`
    DoWhile { body: Box<Stmt>, cond: Expr },
    /// `for (init?; cond?; update?) body`
    For {
        init:   Option<Box<Stmt>>,
        cond:   Option<Expr>,
        update: Vec<Expr>,
        body:   Box<Stmt>,
    },
    /// `for (Type var : iterable) body`
    ForEach {
        ty:       TypeExpr,
        name:     String,
        iterable: Expr,
        body:     Box<Stmt>,
    },
    /// `{ stmts }`
    Block(Block),
    /// `switch (expr) { case x: ... }`
    Switch { expr: Expr, cases: Vec<SwitchCase> },
    /// `throw expr;`
    Throw(Expr),
    /// `try { ... } catch (Type e) { ... } finally { ... }`
    TryCatch {
        try_body:     Block,
        catches:      Vec<CatchClause>,
        finally_body: Option<Block>,
    },
    /// `break;` or `break label;`
    Break(Option<String>),
    /// `continue;` or `continue label;`
    Continue(Option<String>),
    /// `label: stmt`
    Labeled { label: String, stmt: Box<Stmt> },
    /// `synchronized (expr) { ... }`
    Synchronized { expr: Expr, body: Block },
    /// `assert expr;` or `assert expr : message;`
    Assert { expr: Expr, message: Option<Expr> },
    /// `yield expr;` (in switch expressions)
    Yield(Expr),
    /// empty `;`
    Empty,
}

/// A catch clause in a try statement.
#[derive(Debug, Clone)]
pub struct CatchClause {
    pub exception_types: Vec<TypeExpr>,
    pub name:            String,
    pub body:            Block,
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal
    IntLit(i64),
    /// Float literal
    FloatLit(f64),
    /// String literal
    StrLit(String),
    /// Char literal (stored as int for JVM compat)
    CharLit(i64),
    /// Boolean literal
    BoolLit(bool),
    /// null
    Null,
    /// Variable / field reference
    Ident(String),
    /// `this`
    This,
    /// `super`
    Super,
    /// `expr.field`
    Field { obj: Box<Expr>, name: String },
    /// `expr.method(args)` or `method(args)`
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// `new Type(args)` or `new Type(args) { ... }` (anonymous class)
    New { ty: TypeExpr, args: Vec<Expr>, body: Option<Vec<Member>> },
    /// `new Type[len]`
    NewArray { ty: TypeExpr, len: Box<Expr> },
    /// `new Type[m][n]` — multi-dimensional array
    NewMultiArray { ty: TypeExpr, dims: Vec<Expr> },
    /// `new Type[] { ... }` or `{ ... }` array initializer
    ArrayInit { ty: Option<TypeExpr>, elements: Vec<Expr> },
    /// `arr[idx]`
    Index { arr: Box<Expr>, idx: Box<Expr> },
    /// `lhs op rhs`
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// `op expr`
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
    /// `lhs = rhs`
    Assign { lhs: Box<Expr>, rhs: Box<Expr> },
    /// `lhs op= rhs` (compound assignment)
    CompoundAssign { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    /// `(Type) expr`
    Cast { ty: TypeExpr, expr: Box<Expr> },
    /// `cond ? then : else`
    Ternary { cond: Box<Expr>, then: Box<Expr>, else_: Box<Expr> },
    /// `expr instanceof Type`
    Instanceof { expr: Box<Expr>, ty: TypeExpr },
    /// `expr instanceof Type name` (pattern matching, Java 16+)
    InstanceofPattern { expr: Box<Expr>, ty: TypeExpr, name: String },
    /// `(params) -> body`
    Lambda { params: Vec<LambdaParam>, body: Box<LambdaBody> },
    /// `expr::method` or `Type::method` or `Type::new`
    MethodRef { obj: Box<Expr>, name: String },
    /// Switch expression: `switch (expr) { case X -> val; ... }`
    SwitchExpr { expr: Box<Expr>, cases: Vec<SwitchCase> },
}

/// Lambda parameter (may or may not have explicit type).
#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: String,
    pub ty:   Option<TypeExpr>,
}

/// Lambda body — either a single expression or a block.
#[derive(Debug, Clone)]
pub enum LambdaBody {
    Expr(Expr),
    Block(Block),
}

/// A switch case arm.
#[derive(Debug, Clone)]
pub struct SwitchCase {
    /// `None` = default case. Multiple labels for `case 1, 2, 3 ->`.
    pub labels: Option<Vec<Expr>>,
    pub body:   Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor,
    Shl, Shr, UShr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
}
