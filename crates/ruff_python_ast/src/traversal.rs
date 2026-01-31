//! Utilities for manually traversing a Python AST.
use crate::{self as ast, AnyNodeRef, ExceptHandler, Stmt, StmtBody};

/// Given a [`Stmt`] and its parent, return the [`ast::Suite`] that contains the [`Stmt`].
pub fn suite<'a>(
    stmt: impl Into<AnyNodeRef<'a>>,
    parent: impl Into<AnyNodeRef<'a>>,
) -> Option<EnclosingSuite<'a>> {
    // TODO: refactor this to work without a parent, ie when `stmt` is at the top level
    let stmt = stmt.into();
    match parent.into() {
        AnyNodeRef::ModModule(ast::ModModule { body, .. }) => {
            body_slice(body).and_then(|suite| EnclosingSuite::new(suite, stmt))
        }
        AnyNodeRef::StmtFunctionDef(ast::StmtFunctionDef { body, .. }) => {
            body_slice(body).and_then(|suite| EnclosingSuite::new(suite, stmt))
        }
        AnyNodeRef::StmtClassDef(ast::StmtClassDef { body, .. }) => {
            body_slice(body).and_then(|suite| EnclosingSuite::new(suite, stmt))
        }
        AnyNodeRef::StmtFor(ast::StmtFor { body, orelse, .. }) => {
            body_slice(body)
                .and_then(|suite| EnclosingSuite::new(suite, stmt))
                .or_else(|| {
                    body_slice(orelse).and_then(|suite| EnclosingSuite::new(suite, stmt))
                })
        }
        AnyNodeRef::StmtWhile(ast::StmtWhile { body, orelse, .. }) => {
            body_slice(body)
                .and_then(|suite| EnclosingSuite::new(suite, stmt))
                .or_else(|| {
                    body_slice(orelse).and_then(|suite| EnclosingSuite::new(suite, stmt))
                })
        }
        AnyNodeRef::StmtIf(ast::StmtIf {
            body,
            elif_else_clauses,
            ..
        }) => body_slice(body)
            .and_then(|suite| EnclosingSuite::new(suite, stmt))
            .or_else(|| {
                elif_else_clauses
                    .iter()
                    .filter_map(|clause| body_slice(&clause.body))
                    .find_map(|suite| EnclosingSuite::new(suite, stmt))
            }),
        AnyNodeRef::StmtWith(ast::StmtWith { body, .. }) => {
            body_slice(body).and_then(|suite| EnclosingSuite::new(suite, stmt))
        }
        AnyNodeRef::StmtMatch(ast::StmtMatch { cases, .. }) => cases
            .iter()
            .filter_map(|case| body_slice(&case.body))
            .find_map(|body| EnclosingSuite::new(body, stmt)),
        AnyNodeRef::StmtTry(ast::StmtTry {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        }) => body_slice(body)
            .and_then(|suite| EnclosingSuite::new(suite, stmt))
            .or_else(|| {
                handlers
                    .iter()
                    .filter_map(ExceptHandler::as_except_handler)
                    .filter_map(|handler| body_slice(&handler.body))
                    .find_map(|suite| EnclosingSuite::new(suite, stmt))
            })
            .or_else(|| {
                body_slice(orelse).and_then(|suite| EnclosingSuite::new(suite, stmt))
            })
            .or_else(|| {
                body_slice(finalbody).and_then(|suite| EnclosingSuite::new(suite, stmt))
            }),
        _ => None,
    }
}

fn body_slice(body: &StmtBody) -> Option<&[Box<Stmt>]> {
    Some(&body.body)
}

pub struct EnclosingSuite<'a> {
    suite: &'a [Box<Stmt>],
    position: usize,
}

impl<'a> EnclosingSuite<'a> {
    pub fn new(suite: &'a [Box<Stmt>], stmt: AnyNodeRef<'a>) -> Option<Self> {
        let position = suite
            .iter()
            .position(|sibling| AnyNodeRef::ptr_eq(sibling.as_ref().into(), stmt))?;

        Some(EnclosingSuite { suite, position })
    }

    pub fn next_sibling(&self) -> Option<&'a Stmt> {
        self.suite.get(self.position + 1).map(|stmt| stmt.as_ref())
    }

    pub fn next_siblings(&self) -> &'a [Box<Stmt>] {
        self.suite.get(self.position + 1..).unwrap_or_default()
    }

    pub fn previous_sibling(&self) -> Option<&'a Stmt> {
        self.suite
            .get(self.position.checked_sub(1)?)
            .map(|stmt| stmt.as_ref())
    }
}

impl std::ops::Deref for EnclosingSuite<'_> {
    type Target = [Box<Stmt>];

    fn deref(&self) -> &Self::Target {
        self.suite
    }
}
