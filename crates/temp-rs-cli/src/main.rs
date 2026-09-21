use anyhow::Result;
use rustyline::DefaultEditor;
use temp_rs_core::Engine;

fn main() -> Result<()> {
    let mut engine = Engine::new()?;
    let mut rl = DefaultEditor::new()?;

    loop {
        match rl.readline("-> ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(trimmed);

                if let Err(err) = engine.eval(trimmed) {
                    eprintln!("{err}");
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("readline error: {err}");
                break;
            }
        }
    }

    Ok(())
}
