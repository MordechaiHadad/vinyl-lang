use miette::SourceSpan;
use tree_sitter::Node;

use crate::{
    ParserDiagnostic, ast::{
        expression::Expression,
        operator::AssignOp,
        statement::{AssignTarget, Statement},
        types::Type,
    }, lower::{
        Lowerer,
        helpers::{children, node_text},
    }
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_block(&self, node: &Node) -> Result<Vec<Statement>, ParserDiagnostic> {
        let mut stmts = Vec::new();
        let child_count = node.named_child_count();
        for i in 0..child_count {
            if let Some(child) = node.named_child(i as u32) {
                let is_last = i == child_count - 1;
                match self.lower_statement(&child) {
                    Ok(Some(stmt)) => {
                        if is_last {
                            if let Statement::Expression(Expression::If {
                                span: if_span,
                                condition,
                                then_block,
                                else_if,
                                else_block,
                            }) = stmt
                            {
                                stmts.push(Statement::Value(
                                    Expression::If {
                                        span: if_span,
                                        condition,
                                        then_block,
                                        else_if,
                                        else_block,
                                    },
                                    if_span,
                                ));
                            } else {
                                stmts.push(stmt);
                            }
                        } else {
                            stmts.push(stmt);
                        }
                    }
                    Ok(None) => {
                        if let Ok(expr) = self.lower_expression(&child) {
                            let span = SourceSpan::from(child.start_byte()..child.end_byte());
                            stmts.push(Statement::Value(expr, span));
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(stmts)
    }

    pub(super) fn lower_statement(&self, node: &Node) -> Result<Option<Statement>, ParserDiagnostic> {
        match node.kind() {
            "let_declaration" => self.lower_let(node).map(Some),
            "assignment_statement" => self.lower_assignment(node).map(Some),
            "return_statement" => self.lower_return(node).map(Some),
            "expression_statement" => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i as u32) {
                        return self
                            .lower_expression(&child)
                            .map(|e| Some(Statement::Expression(e)));
                    }
                }
                Err(self.span_error(node, "empty expression statement"))
            }
            "if_expression" => self.lower_if(node).map(|e| Some(Statement::Expression(e))),
            "while_statement" => {
                let span = SourceSpan::from(node.start_byte()..node.end_byte());
                let named = children(node);
                let condition = match named.first() {
                    Some(n) => self.lower_expression(n)?,
                    None => {
                        return Err(self.span_error(node, "incomplete while: missing condition"));
                    }
                };
                let body = match named.get(1) {
                    Some(n) => self.lower_block(n)?,
                    None => {
                        return Err(self.span_error(node, "incomplete while: missing body"));
                    }
                };
                let break_stmt = Statement::Break(span);
                let if_expr = Expression::If {
                    span,
                    condition: Box::new(condition),
                    then_block: body,
                    else_if: Vec::new(),
                    else_block: Some(vec![break_stmt]),
                };
                Ok(Some(Statement::Loop {
                    span,
                    body: vec![Statement::Expression(if_expr)],
                }))
            }
            "loop_statement" => {
                let span = SourceSpan::from(node.start_byte()..node.end_byte());
                let body = match node.named_child(0) {
                    Some(n) => self.lower_block(&n)?,
                    None => {
                        return Err(self.span_error(node, "incomplete loop: missing body"));
                    }
                };
                Ok(Some(Statement::Loop { span, body }))
            }
            "break_statement" => {
                let span = SourceSpan::from(node.start_byte()..node.end_byte());
                Ok(Some(Statement::Break(span)))
            }
            "continue_statement" => {
                let span = SourceSpan::from(node.start_byte()..node.end_byte());
                Ok(Some(Statement::Continue(span)))
            }
            _ => Ok(None),
        }
    }

    pub(super) fn lower_let(&self, node: &Node) -> Result<Statement, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let mutable = node.child_by_field_name("mut").is_some();
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let type_ = self.find_type_annotation(node)?;
        let value = self.lower_any_expression(node)?;
        Ok(Statement::Let {
            span,
            name,
            mutable,
            type_,
            value,
        })
    }

    pub(super) fn find_type_annotation(&self, node: &Node) -> Result<Option<Type>, ParserDiagnostic> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() == "type_annotation"
            {
                return self.lower_type(&child).map(Some);
            }
        }
        Ok(None)
    }

    pub(super) fn lower_return(&self, node: &Node) -> Result<Statement, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                return self
                    .lower_expression(&child)
                    .map(|e| Statement::Return(Some(e), span));
            }
        }
        Ok(Statement::Return(None, span))
    }

    pub(super) fn lower_assignment(&self, node: &Node) -> Result<Statement, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let named = children(node);
        if named.len() < 2 {
            return Err(self.span_error(node, "incomplete assignment"));
        }

        let target = self.lower_assign_target(&named[0])?;
        let op = self.lower_assign_op(node)?;
        let value = self.lower_expression(&named[1])?;

        Ok(Statement::Assign {
            span,
            target,
            op,
            value: Box::new(value),
        })
    }

    pub(super) fn lower_assign_target(&self, node: &Node) -> Result<AssignTarget, ParserDiagnostic> {
        let span = || SourceSpan::from(node.start_byte()..node.end_byte());
        match node.kind() {
            "value_identifier" | "type_identifier" => {
                Ok(AssignTarget::Ident(node_text(node, self.source), span()))
            }
            "index_expression" => {
                let children = children(node);
                if children.len() < 2 {
                    return Err(self.span_error(node, "incomplete index expression in assignment"));
                }
                let array = self.lower_expression(&children[0])?;
                let index = self.lower_expression(&children[1])?;
                Ok(AssignTarget::Index {
                    span: span(),
                    array: Box::new(array),
                    index: Box::new(index),
                })
            }
            "field_access_expression" => {
                let children = children(node);
                if children.len() < 2 {
                    return Err(self.span_error(node, "incomplete field access in assignment"));
                }
                let object = self.lower_expression(&children[0])?;
                let name = node_text(&self.child_by_field(node, "field")?, self.source);
                Ok(AssignTarget::Field {
                    span: span(),
                    object: Box::new(object),
                    name,
                })
            }
            _ => Err(self.span_error(
                node,
                &format!("invalid assignment target: `{}`", node.kind()),
            )),
        }
    }

    pub(super) fn lower_assign_op(&self, node: &Node) -> Result<AssignOp, ParserDiagnostic> {
        let op_node = node
            .child_by_field_name("operator")
            .ok_or_else(|| self.span_error(node, "missing assignment operator"))?;
        let text = node_text(&op_node, self.source);
        Ok(match text.as_str() {
            "=" => AssignOp::Eq,
            "+=" => AssignOp::AddEq,
            "-=" => AssignOp::SubEq,
            "*=" => AssignOp::MulEq,
            "/=" => AssignOp::DivEq,
            "%=" => AssignOp::RemEq,
            "&=" => AssignOp::BitAndEq,
            "|=" => AssignOp::BitOrEq,
            "^=" => AssignOp::BitXorEq,
            "<<=" => AssignOp::ShlEq,
            ">>=" => AssignOp::ShrEq,
            other => {
                return Err(
                    self.span_error(&op_node, &format!("unknown assignment operator `{other}`"))
                );
            }
        })
    }
}
