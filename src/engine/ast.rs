//! AST (Abstract Syntax Tree)
//!
//! Abstract syntax tree
//!
//! This module defines the AST node structures used in PHP compilation

use crate::engine::types::{PhpString, Val};

/// AST node kind
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstKind {
    // Special nodes
    AstNone = 0,
    AstVal = 1,
    AstZnode = 2,

    // List nodes
    AstList = 128,
    AstArrayDecl = 129,
    AstEncapsList = 130,

    // Declaration nodes
    AstName = 256,
    AstClass = 257,
    AstMethod = 258,
    AstFuncDecl = 259,
    AstClosure = 260,
    AstClosureUse = 261,
    AstParamList = 262,
    AstParam = 263,
    AstStmtList = 264,
    AstPropertyList = 265,
    AstProperty = 266,
    AstConstDecl = 267,
    AstConstElem = 268,

    // Statement nodes
    AstIf = 512,
    AstSwitch = 513,
    AstSwitchList = 514,
    AstSwitchCase = 515,
    AstWhile = 516,
    AstDoWhile = 517,
    AstFor = 518,
    AstForeach = 519,
    AstDeclare = 520,
    AstTry = 521,
    AstCatch = 522,
    AstFinally = 523,
    AstThrowStmt = 524,
    AstGoto = 525,
    AstLabel = 526,
    AstBreak = 527,
    AstContinue = 528,
    AstReturn = 529,
    AstYieldStmt = 530,
    AstYieldFromStmt = 531,
    AstGlobal = 532,
    AstUnset = 533,
    AstEcho = 534,
    AstStatic = 535,
    AstInlineHtml = 537,

    // Expression nodes
    AstMagicConst = 768,
    AstMethodCall = 769,
    AstStaticCall = 770,
    AstConditional = 771,
    AstEmpty = 772,
    AstIsset = 773,
    AstSilence = 774,
    AstThrowExpr = 775,
    AstYieldExpr = 776,
    AstYieldFromExpr = 777,
    AstCoalesce = 778,
    AstUnaryPlus = 779,
    AstUnaryMinus = 780,
    AstCast = 781,
    AstPlus = 782,
    AstMinus = 783,
    AstMul = 784,
    AstDiv = 785,
    AstMod = 786,
    AstPow = 787,
    AstConcat = 788,
    AstShl = 789,
    AstShr = 790,
    AstSmaller = 791,
    AstSmallerOrEqual = 792,
    AstGreater = 793,
    AstGreaterOrEqual = 794,
    AstEqual = 795,
    AstNotEqual = 796,
    AstIdentical = 797,
    AstNotIdentical = 798,
    AstSpaceship = 799,
    AstAnd = 800,
    AstOr = 801,
    AstXor = 802,
    AstBooleanAnd = 803,
    AstBooleanOr = 804,
    AstBooleanNot = 805,
    AstBitwiseNot = 806,
    AstAssign = 807,
    AstAssignRef = 808,
    AstAssignOp = 809,
    AstAssignConcat = 810,
    AstPostInc = 811,
    AstPreInc = 812,
    AstPostDec = 813,
    AstPreDec = 814,
    AstArrayExpr = 815,
    AstArrayElem = 816,
    AstNew = 817,
    AstInstanceof = 818,
    AstClone = 819,
    AstAssignDim = 820,
    AstAssignObj = 821,
    AstAssignNew = 822,
    AstAssignStaticProp = 823,
    AstAssignClassConst = 824,
    AstDim = 829,
    AstPropertyAccess = 830,
    AstStaticProperty = 831,
    AstCall = 832,
    AstClassConst = 833,
    AstVar = 834,
    AstConst = 835,
}

/// AST node flags
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstFlags {
    None = 0,
    Children1 = 1,
    Children2 = 2,
    Children3 = 3,
    Children4 = 4,
    Children5 = 5,
    Children6 = 6,
    Children7 = 7,
    Children8 = 8,
    Children9 = 9,
    Children10 = 10,
    Children11 = 11,
    Children12 = 12,
    Children13 = 13,
    Children14 = 14,
    Children15 = 15,
}

/// AST node structure
#[derive(Debug)]
pub struct Ast {
    pub kind: AstKind,
    pub flags: u16,
    pub attr: u16,
    pub lineno: u32,
    pub children: Vec<AstNode>,
}

/// AST node (can be AST or Val)
#[derive(Debug)]
pub enum AstNode {
    Ast(Box<Ast>),
    Val(Val),
    String(Box<PhpString>),
    None,
}

impl Ast {
    pub fn new(kind: AstKind, flags: u16, lineno: u32) -> Self {
        Self {
            kind,
            flags,
            attr: 0,
            lineno,
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<AstNode>) -> Self {
        self.children = children;
        self.flags = self.children.len() as u16;
        self
    }

    pub fn add_child(&mut self, child: AstNode) {
        self.children.push(child);
        self.flags = self.children.len() as u16;
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn get_child(&self, index: usize) -> Option<&AstNode> {
        self.children.get(index)
    }
}

impl Default for Ast {
    fn default() -> Self {
        Self::new(AstKind::AstVal, 0, 0)
    }
}
