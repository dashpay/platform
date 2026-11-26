//! The code points translating a pattern may case-fold, estimated from its syntax tree.

use regex_syntax::ast::{self, Ast, ClassSetBinaryOp, ClassSetItem, Flag, Flags, GroupKind};
use std::convert::Infallible;

/// Every code point, which a class naming a Unicode property, a Perl class (`\d`, `\s`, `\w`
/// and their negations) and a negated class count as, whatever they hold.
const EVERY_CODE_POINT: u64 = 0x11_0000;

/// The most code points an ASCII class (`[:alpha:]` and the others) holds.
const ASCII_CODE_POINTS: u64 = 128;

/// An estimate of the code points the `regex` crate case-folds when it translates `ast` to the
/// expression it compiles, or 0 when no flag of the pattern turns case-insensitive matching on.
/// It is not an exact count: a class gathered into the class around it is counted at the span
/// it had before it was folded, which folding can widen, and a literal outside a class is not
/// counted. The walk it bounds stays within a small multiple of it.
///
/// Under case-insensitive matching the translator walks every code point of a class to add the
/// ones that fold to it, before it negates the class: each class naming a Unicode property, each
/// ASCII class, each bracketed class once its items are gathered (a nested one too, and again
/// for each bracketed class around it), and both operands of a class set operation (`&&`, `--`,
/// `~~`). That walk, not the size of the compiled program, is what translating a short pattern
/// costs. Here a class spans the code points of its items, a literal one and a range those from
/// its start to its end, and a class naming a Unicode property, a Perl class or a negated class
/// spans every code point. Each place the translator folds a class adds the class's span. The
/// whole pattern counts as case-insensitive once any of its flags turns `i` on, wherever a later
/// flag turns it off again.
pub(super) fn case_folded_code_points(ast: &Ast) -> u64 {
    match ast::visit(ast, CaseFolding::default()) {
        Ok(code_points) => code_points,
        Err(never) => match never {},
    }
}

#[derive(Default)]
struct CaseFolding {
    case_insensitive: bool,
    /// The code points counted as folded so far.
    folded: u64,
    /// The span of each class being gathered, the innermost last.
    spans: Vec<u64>,
    /// The span of the left operand of each class set operation whose right operand is being
    /// gathered, the innermost last.
    left_operands: Vec<u64>,
}

impl CaseFolding {
    fn read_flags(&mut self, flags: &Flags) {
        if flags.flag_state(Flag::CaseInsensitive) == Some(true) {
            self.case_insensitive = true;
        }
    }

    fn fold(&mut self, span: u64) {
        self.folded = self.folded.saturating_add(span);
    }

    /// Adds `span` to the class being gathered.
    fn gather(&mut self, span: u64) {
        if let Some(class) = self.spans.last_mut() {
            *class = class.saturating_add(span).min(EVERY_CODE_POINT);
        }
    }

    fn gathered(&mut self) -> u64 {
        self.spans.pop().unwrap_or_default()
    }
}

impl ast::Visitor for CaseFolding {
    type Output = u64;
    type Err = Infallible;

    fn finish(self) -> Result<u64, Infallible> {
        Ok(if self.case_insensitive {
            self.folded
        } else {
            0
        })
    }

    fn visit_pre(&mut self, ast: &Ast) -> Result<(), Infallible> {
        match ast {
            Ast::Flags(set_flags) => self.read_flags(&set_flags.flags),
            Ast::Group(group) => {
                if let GroupKind::NonCapturing(flags) = &group.kind {
                    self.read_flags(flags);
                }
            }
            Ast::ClassUnicode(_) => self.fold(EVERY_CODE_POINT),
            Ast::ClassBracketed(_) => self.spans.push(0),
            _ => {}
        }
        Ok(())
    }

    fn visit_post(&mut self, ast: &Ast) -> Result<(), Infallible> {
        if let Ast::ClassBracketed(_) = ast {
            let span = self.gathered();
            self.fold(span);
        }
        Ok(())
    }

    fn visit_class_set_item_pre(&mut self, item: &ClassSetItem) -> Result<(), Infallible> {
        if let ClassSetItem::Bracketed(_) = item {
            self.spans.push(0);
        }
        Ok(())
    }

    fn visit_class_set_item_post(&mut self, item: &ClassSetItem) -> Result<(), Infallible> {
        match item {
            ClassSetItem::Empty(_) | ClassSetItem::Union(_) => {}
            ClassSetItem::Literal(_) => self.gather(1),
            ClassSetItem::Range(range) => self.gather(
                u64::from(range.end.c)
                    .saturating_sub(u64::from(range.start.c))
                    .saturating_add(1),
            ),
            ClassSetItem::Ascii(ascii) => {
                self.fold(ASCII_CODE_POINTS);
                self.gather(if ascii.negated {
                    EVERY_CODE_POINT
                } else {
                    ASCII_CODE_POINTS
                });
            }
            ClassSetItem::Unicode(_) => {
                self.fold(EVERY_CODE_POINT);
                self.gather(EVERY_CODE_POINT);
            }
            ClassSetItem::Perl(_) => self.gather(EVERY_CODE_POINT),
            ClassSetItem::Bracketed(bracketed) => {
                let span = self.gathered();
                self.fold(span);
                self.gather(if bracketed.negated {
                    EVERY_CODE_POINT
                } else {
                    span
                });
            }
        }
        Ok(())
    }

    fn visit_class_set_binary_op_pre(&mut self, _: &ClassSetBinaryOp) -> Result<(), Infallible> {
        self.spans.push(0);
        Ok(())
    }

    fn visit_class_set_binary_op_in(&mut self, _: &ClassSetBinaryOp) -> Result<(), Infallible> {
        let left = self.gathered();
        self.left_operands.push(left);
        self.spans.push(0);
        Ok(())
    }

    fn visit_class_set_binary_op_post(&mut self, _: &ClassSetBinaryOp) -> Result<(), Infallible> {
        let right = self.gathered();
        let left = self.left_operands.pop().unwrap_or_default();
        let span = left.saturating_add(right);
        self.fold(span);
        self.gather(span.min(EVERY_CODE_POINT));
        Ok(())
    }
}
