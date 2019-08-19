//!
//! The expression parser.
//!

mod add_sub;
mod and;
mod casting;
mod comparison;
mod mul_div_rem;
mod or;
mod xor;

use std::cell::RefCell;
use std::rc::Rc;

use crate::lexical::Lexeme;
use crate::lexical::Symbol;
use crate::lexical::Token;
use crate::lexical::TokenStream;
use crate::syntax::Expression;
use crate::syntax::ExpressionOperator;
use crate::Error;

use self::add_sub::Parser as AddSubOperandParser;
use self::and::Parser as AndOperandParser;
use self::casting::Parser as CastingOperandParser;
use self::comparison::Parser as ComparisonOperandParser;
use self::mul_div_rem::Parser as MulDivRemOperandParser;
use self::or::Parser as OrOperandParser;
use self::xor::Parser as XorOperandParser;

#[derive(Debug, Clone, Copy)]
pub enum State {
    LogicalOrOperand,
    LogicalOrOperator,
    End,
}

impl Default for State {
    fn default() -> Self {
        State::LogicalOrOperand
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,
    expression: Expression,
    operator: Option<(ExpressionOperator, Token)>,
}

impl Parser {
    pub fn parse(mut self, stream: Rc<RefCell<TokenStream>>) -> Result<Expression, Error> {
        loop {
            match self.state {
                State::LogicalOrOperand => {
                    let rpn = OrOperandParser::default().parse(stream.clone())?;
                    self.expression.append(rpn);
                    if let Some(operator) = self.operator.take() {
                        self.expression.push_operator(operator);
                    }
                    self.state = State::LogicalOrOperator;
                }
                State::LogicalOrOperator => {
                    let peek = stream.borrow_mut().peek();
                    match peek {
                        Some(Ok(Token {
                            lexeme: Lexeme::Symbol(Symbol::DoubleVerticalBar),
                            ..
                        })) => {
                            let token = stream.borrow_mut().next().unwrap().unwrap();
                            self.operator = Some((ExpressionOperator::Or, token));
                            self.state = State::LogicalOrOperand;
                        }
                        _ => self.state = State::End,
                    }
                }
                State::End => return Ok(self.expression),
            }
        }
    }
}
