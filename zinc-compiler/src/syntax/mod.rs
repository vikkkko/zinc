//!
//! The syntax parser.
//!

mod error;
mod parser;
mod tests;
mod tree;

pub use self::error::Error;
pub use self::parser::take_or_next;
pub use self::parser::AccessOperandParser;
pub use self::parser::AddSubOperandParser;
pub use self::parser::AndOperandParser;
pub use self::parser::ArrayExpressionParser;
pub use self::parser::BindingPatternListParser;
pub use self::parser::BindingPatternParser;
pub use self::parser::BlockExpressionParser;
pub use self::parser::CastingOperandParser;
pub use self::parser::ComparisonOperandParser;
pub use self::parser::ConditionalExpressionParser;
pub use self::parser::ConstStatementParser;
pub use self::parser::EnumStatementParser;
pub use self::parser::ExpressionListParser;
pub use self::parser::ExpressionParser;
pub use self::parser::FieldListParser;
pub use self::parser::FieldParser;
pub use self::parser::FnStatementParser;
pub use self::parser::FunctionLocalStatementParser;
pub use self::parser::ImplStatementParser;
pub use self::parser::ImplementationLocalStatementParser;
pub use self::parser::LetStatementParser;
pub use self::parser::LoopStatementParser;
pub use self::parser::MatchExpressionParser;
pub use self::parser::MatchPatternParser;
pub use self::parser::ModStatementParser;
pub use self::parser::ModuleLocalStatementParser;
pub use self::parser::MulDivRemOperandParser;
pub use self::parser::OrOperandParser;
pub use self::parser::Parser;
pub use self::parser::PathOperandParser;
pub use self::parser::RangeOperandParser;
pub use self::parser::StaticStatementParser;
pub use self::parser::StructStatementParser;
pub use self::parser::StructureExpressionParser;
pub use self::parser::TerminalOperandParser;
pub use self::parser::TupleExpressionParser;
pub use self::parser::TypeParser;
pub use self::parser::TypeStatementParser;
pub use self::parser::UseStatementParser;
pub use self::parser::VariantListParser;
pub use self::parser::VariantParser;
pub use self::parser::XorOperandParser;
pub use self::tree::ArrayExpression;
pub use self::tree::ArrayExpressionBuilder;
pub use self::tree::BindingPattern;
pub use self::tree::BindingPatternBuilder;
pub use self::tree::BindingPatternVariant;
pub use self::tree::BlockExpression;
pub use self::tree::BlockExpressionBuilder;
pub use self::tree::BooleanLiteral;
pub use self::tree::ConditionalExpression;
pub use self::tree::ConditionalExpressionBuilder;
pub use self::tree::ConstStatement;
pub use self::tree::ConstStatementBuilder;
pub use self::tree::EnumStatement;
pub use self::tree::EnumStatementBuilder;
pub use self::tree::Expression;
pub use self::tree::ExpressionAuxiliary;
pub use self::tree::ExpressionBuilder;
pub use self::tree::ExpressionElement;
pub use self::tree::ExpressionObject;
pub use self::tree::ExpressionOperand;
pub use self::tree::ExpressionOperator;
pub use self::tree::Field;
pub use self::tree::FieldBuilder;
pub use self::tree::FnStatement;
pub use self::tree::FnStatementBuilder;
pub use self::tree::FunctionLocalStatement;
pub use self::tree::Identifier;
pub use self::tree::IdentifierBuilder;
pub use self::tree::ImplStatement;
pub use self::tree::ImplStatementBuilder;
pub use self::tree::ImplementationLocalStatement;
pub use self::tree::IntegerLiteral;
pub use self::tree::LetStatement;
pub use self::tree::LetStatementBuilder;
pub use self::tree::LoopStatement;
pub use self::tree::LoopStatementBuilder;
pub use self::tree::MatchExpression;
pub use self::tree::MatchExpressionBuilder;
pub use self::tree::MatchPattern;
pub use self::tree::MatchPatternBuilder;
pub use self::tree::MatchPatternVariant;
pub use self::tree::MemberInteger;
pub use self::tree::MemberIntegerBuilder;
pub use self::tree::MemberString;
pub use self::tree::MemberStringBuilder;
pub use self::tree::ModStatement;
pub use self::tree::ModStatementBuilder;
pub use self::tree::ModuleLocalStatement;
pub use self::tree::StaticStatement;
pub use self::tree::StaticStatementBuilder;
pub use self::tree::StringLiteral;
pub use self::tree::StructStatement;
pub use self::tree::StructStatementBuilder;
pub use self::tree::StructureExpression;
pub use self::tree::StructureExpressionBuilder;
pub use self::tree::SyntaxTree;
pub use self::tree::TupleExpression;
pub use self::tree::TupleExpressionBuilder;
pub use self::tree::Type;
pub use self::tree::TypeBuilder;
pub use self::tree::TypeStatement;
pub use self::tree::TypeStatementBuilder;
pub use self::tree::TypeVariant;
pub use self::tree::UseStatement;
pub use self::tree::UseStatementBuilder;
pub use self::tree::Variant;
pub use self::tree::VariantBuilder;

static PANIC_BUILDER_REQUIRES_VALUE: &str = "The builder requires a value: ";
static PANIC_BUILDER_TYPE: &str = "The type building logic is verified";
