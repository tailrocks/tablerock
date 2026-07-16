use std::process::ExitCode;

fn main() -> ExitCode {
    match tablerock_cli::run_caught() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("TableRock: {error}");
            ExitCode::FAILURE
        }
    }
}
