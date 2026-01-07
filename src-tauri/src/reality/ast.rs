use rowan::ast::AstNode;
use super::syntax::{RealityLanguage, SyntaxKind};

pub type SyntaxNode = rowan::SyntaxNode<RealityLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<RealityLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<RealityLanguage>;

macro_rules! reality_ast {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            syntax: SyntaxNode,
        }
        impl AstNode for $name {
            type Language = RealityLanguage;
            fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool {
                kind == SyntaxKind::$kind
            }
            fn cast(syntax: SyntaxNode) -> Option<Self> {
                if Self::can_cast(syntax.kind()) {
                    Some(Self { syntax })
                } else {
                    None
                }
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

reality_ast!(Document, Document);
reality_ast!(Paragraph, Paragraph);
reality_ast!(EntitySpan, EntitySpan);
