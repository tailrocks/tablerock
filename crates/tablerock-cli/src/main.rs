use std::process::ExitCode;

fn main() -> ExitCode {
    if matches!(std::env::args().nth(1).as_deref(), Some("--version" | "-V")) {
        println!("tablerock {}", env!("TABLEROCK_VERSION"));
        return ExitCode::SUCCESS;
    }

    match tablerock_cli::run_caught() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("TableRock: {error}");
            ExitCode::FAILURE
        }
    }
}
