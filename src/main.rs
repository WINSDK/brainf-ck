use std::fs;

macro_rules! exit {
    () => {
        std::process::exit(1);
    };
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
        std::process::exit(1);
    }};
}

trait Indentation {
    fn push_indented(&mut self, line: &str);
    fn push_label(&mut self, label: &str);
}

impl Indentation for String {
    fn push_indented(&mut self, line: &str) {
        self.push_str("    ");
        self.push_str(line);
        self.push('\n');
    }

    fn push_label(&mut self, label: &str) {
        self.push('_');
        self.push_str(label);
        self.push_str(":\n");
    }
}

fn main() -> std::io::Result<()> {
    let mut files = std::env::args()
        .skip(1)
        .filter(|p| p.ends_with("b"))
        .map(std::path::PathBuf::from)
        .peekable();

    if files.peek().is_none() {
        exit!("brainf*ck error: no input files");
    }

    while let Some(file_name) = files.next() {
        let mut instructions = String::new();
        let syntax = match fs::read(&file_name) {
            Ok(data) => data,
            Err(..) => exit!("No such file or directory: {file_name:?}"),
        };

        // Section header
        instructions.push_str("section .text\n");
        instructions.push_indented("extern _putchar");
        instructions.push_indented("extern _getchar");
        instructions.push_indented("global _main");

        instructions.push_label("main");

        // Allign stack
        instructions.push_indented("push rbp");
        instructions.push_indented("sub rsp, 4096");
        instructions.push_indented("mov rbp, rsp");

        // Zero out the stack
        instructions.push_indented("xor r8, r8");
        instructions.push_label("zeroed");
        instructions.push_indented("mov qword [rbp + r8], 0");
        instructions.push_indented("add r8, 8");
        instructions.push_indented("test r8, 4096");
        instructions.push_indented("jne _zeroed");

        let mut while_loop_stack = Vec::new();
        let mut labels_disambiguator = 1;
        let mut idx = 0;

        while idx < syntax.len() {
            let chr = syntax[idx];
            let mut count = 1;

            // Count if there are duplicates instructions, that can also be simplified.
            if [b'>', b'<', b'+', b'-'].contains(&chr) {
                while syntax.get(idx + 1) == Some(&chr) {
                    count += 1;
                    idx += 1;
                }
            }

            match chr {
                b'[' => {
                    instructions.push_label(&format!("_L{}", labels_disambiguator));
                    instructions.push_indented("mov cl, [rbp]");
                    instructions.push_indented("test cl, cl");

                    instructions.push_indented(&format!("je __L{}", labels_disambiguator + 1));

                    while_loop_stack.push(labels_disambiguator);
                    labels_disambiguator += 2;
                }
                b']' => {
                    let label = while_loop_stack.pop().expect("Mismatched `[..]`");

                    instructions.push_indented(&format!("jmp __L{}", label));
                    instructions.push_label(&format!("_L{}", label + 1));
                }
                b'.' => {
                    instructions.push_indented("movzx rdi, byte [rbp]");
                    instructions.push_indented("call _putchar");
                }
                b',' => {
                    instructions.push_indented("call _getchar");
                    instructions.push_indented("mov byte [rbp], ah");
                }
                b'>' => instructions.push_indented(&format!("add rbp, {count}")),
                b'<' => instructions.push_indented(&format!("sub rbp, {count}")),
                b'+' => {
                    instructions.push_indented(&format!("add byte [rbp], {count}"));
                }
                b'-' => {
                    instructions.push_indented(&format!("sub byte [rbp], {count}"));
                }
                _ => {}
            }

            idx += 1;
        }

        // Newline
        instructions.push_indented("mov rdi, 10");
        instructions.push_indented("call _putchar");

        // Align stack
        instructions.push_indented("add rsp, 4096");
        instructions.push_indented("pop rbp");

        instructions.push_indented("ret");

        let asm_output = file_name.with_extension("asm");
        fs::write(&asm_output, instructions)?;

        let output = std::process::Command::new("nasm")
            .args(&["-f", "macho64"])
            .arg(asm_output)
            .spawn()?
            .wait_with_output()?;

        if !output.status.success() {
            exit!("{}", String::from_utf8_lossy(&output.stderr));
        }

        let output = std::process::Command::new("clang")
            .args(&["-arch", "x86_64"])
            .args(&["-o", file_name.file_stem().and_then(|x| x.to_str()).unwrap_or("main")])
            .arg(file_name.with_extension("o"))
            .spawn()?
            .wait_with_output()?;

        if !output.status.success() {
            exit!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
