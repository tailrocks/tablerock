use std::process::ExitCode;

fn main() -> ExitCode {
    let argument = std::env::args().nth(1);
    if matches!(argument.as_deref(), Some("--version" | "-V")) {
        println!("tablerock {}", env!("TABLEROCK_VERSION"));
        return ExitCode::SUCCESS;
    }
    if matches!(argument.as_deref(), Some("--support-bundle")) {
        let bundle = tablerock_core::SupportBundle::new(tablerock_core::SupportPlatform::current());
        print!("{}", bundle.render(env!("TABLEROCK_VERSION")));
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
