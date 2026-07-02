use board_lexer as board;
use clap::Parser;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(clap::Parser)]
struct Cli {
    /// The path to the file to read
    path: Option<std::path::PathBuf>,

    /// Runs the lexer (json output)
    #[arg(short = 'l', long = "lexer")]
    lexer: bool,

    /// Runs the program (website output)
    #[arg(short = 'r', long = "run")]
    run: bool,

    /// Shows the version
    #[arg(short = 'v', long = "version")]
    version: bool,

    /// Displays documentation
    #[arg(short = 'd', long = "doc")]
    doc: bool,

    /// Updates the program to the latest version
    #[arg(short = 'u', long = "update")]
    update: bool,

    /// Shows config directory
    #[arg(long = "config-dir")]
    config: bool,

    /// Clears code
    #[arg(short = 'c', long = "clear-code")]
    clear_code: bool,
}

fn main() {
    let args = Cli::parse();

    if args.lexer {
        let content = get_file_contents(&args);

        let lexer = board::Lexer::new(&content);
        let json = lexer.get_json(false, false);

        println!(
            "{}",
            serde_json::to_string_pretty(&json).expect("failed to serialize JSON")
        );
    }

    if args.clear_code {
        let content = get_file_contents(&args);

        let lexer = board::Lexer::new(&content);
        let code = lexer.clear_code(false);

        println!("{}", code);
    }

    if args.config {
        let config_path = get_config_path();
        println!("config path: {}", config_path);
        std::process::exit(0);
    }

    if args.doc {
        display_doc();
        std::process::exit(0);
    }

    if args.update {
        let config_path = get_config_path();
        println!("config path: {}", config_path);
        std::process::exit(0);
    }

    if args.version {
        display_version();
        std::process::exit(0);
    }
}

fn get_config_path() -> String {
    if cfg!(target_os = "windows") {
        return "C:\\Users\\.config\\board-lang".to_string();
    } else {
        return "~/.config/board-lang".to_string();
    }
}

fn get_file_contents(args: &Cli) -> String {
    if let Some(path) = &args.path {
        if path.as_os_str().is_empty() {
            eprintln!("NO PATH PROVIDED");
            std::process::exit(1);
        } else if !path.exists() || !path.is_file() {
            eprintln!("FILE NOT FOUND");
            std::process::exit(1);
        } else {
            let contents = std::fs::read_to_string(&path).expect("could not read file");
            if contents.trim().is_empty() || contents == "could not read file" {
                eprintln!("FILE IS EMPTY");
                std::process::exit(1);
            }
            contents
        }
    } else {
        eprintln!("NO PATH PROVIDED");
        std::process::exit(1);
    }
}

fn display_doc() {
    let version = get_installed_version();
    let url = "https://tugalaturtuga.github.io/home/docs/board/?version=".to_owned() + &version;

    // Try opening browser first
    if webbrowser::open(&url).is_ok() {
        println!("Opened documentation in browser");
        return; // success → stop here
    }
    println!("Couldn't open documentation in browser. Do you have an internet connection?");
}

fn get_installed_version() -> String {
    let content = std::fs::read_to_string("cargo.toml").expect("could not read file");
    let parsed: toml::Value = toml::from_str(&content).expect("could not parse cargo.toml");

    let version: &str =
        &("v".to_owned() + parsed["package"]["version"].as_str().unwrap_or("_._._"));
    version.to_string()
}

fn display_version() {
    let version = get_installed_version();
    println!("Board-lang {}", version);
}
