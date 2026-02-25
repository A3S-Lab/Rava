#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{ast::SourceFile, lexer::Lexer, parser::Parser};

    fn lower(src: &str) -> rava_rir::RirModule {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let file = Parser::new(tokens).parse_file().unwrap();
        Lowerer::new("test").lower_file(&file).unwrap()
    }

    #[test]
    fn lower_hello_world_produces_functions() {
        let src = r#"
            class Main {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "Main.main");
    }

    #[test]
    fn lower_arithmetic() {
        let src = r#"
            class Calc {
                int add(int a, int b) { return a + b; }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let instrs = &module.functions[0].basic_blocks[0].instrs;
        assert!(instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::BinOp { .. })));
        assert!(instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::Return(_))));
    }

    #[test]
    fn lower_do_while() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    do { i = i + 1; } while (i < 10);
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        assert!(module.functions[0].basic_blocks.len() >= 3);
    }

    #[test]
    fn lower_break_continue() {
        let src = r#"
            class T {
                void f() {
                    int i = 0;
                    while (i < 10) {
                        if (i == 5) break;
                        i = i + 1;
                        continue;
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        let jump_count = all_instrs.iter().filter(|i| matches!(i, rava_rir::RirInstr::Jump(_))).count();
        assert!(jump_count >= 2, "expected at least 2 jumps for break/continue");
    }

    #[test]
    fn lower_ternary_branches() {
        let src = r#"
            class T {
                int f(int x) {
                    return x > 0 ? x : -x;
                }
            }
        "#;
        let module = lower(src);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        assert!(all_instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::Branch { .. })));
    }

    #[test]
    fn lower_for_each() {
        let src = r#"
            class T {
                void f(String[] items) {
                    for (String s : items) {
                        System.out.println(s);
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        let has_iterator_call = all_instrs.iter().any(|i| {
            if let rava_rir::RirInstr::Call { func, .. } = i {
                func.0 == encode_builtin("__method__iterator")
                    || func.0 == encode_builtin("__method__hasNext")
                    || func.0 == encode_builtin("__method__next")
            } else { false }
        });
        assert!(has_iterator_call, "for-each should use iterator pattern");
    }

    #[test]
    fn lower_switch_type_pattern() {
        let src = r#"
            class T {
                void f(Object obj) {
                    switch (obj) {
                        case String s -> System.out.println(s);
                        case Integer i -> System.out.println(i);
                        default -> {}
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        // Type pattern must emit Instanceof checks
        assert!(
            all_instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::Instanceof { .. })),
            "type pattern switch should emit Instanceof"
        );
    }

    #[test]
    fn lower_switch_case_null() {
        let src = r#"
            class T {
                void f(String s) {
                    switch (s) {
                        case null -> System.out.println("null");
                        default -> System.out.println(s);
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        // case null must emit a ConstNull + Eq check
        assert!(
            all_instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::ConstNull { .. })),
            "case null should emit ConstNull"
        );
    }

    #[test]
    fn lower_switch_guarded_pattern() {
        let src = r#"
            class T {
                void f(Object obj) {
                    switch (obj) {
                        case Integer i when i > 0 -> System.out.println("positive");
                        default -> {}
                    }
                }
            }
        "#;
        let module = lower(src);
        assert_eq!(module.functions.len(), 1);
        let all_instrs: Vec<_> = module.functions[0].basic_blocks.iter()
            .flat_map(|b| &b.instrs).collect();
        // Guarded pattern: Instanceof + Branch for guard short-circuit
        assert!(
            all_instrs.iter().any(|i| matches!(i, rava_rir::RirInstr::Instanceof { .. })),
            "guarded pattern should emit Instanceof"
        );
        let branch_count = all_instrs.iter()
            .filter(|i| matches!(i, rava_rir::RirInstr::Branch { .. }))
            .count();
        assert!(branch_count >= 2, "guarded pattern needs at least 2 branches (instanceof + guard)");
    }
}
