//! `keinontolibrary` CLI — `decline`, `paradigm`, `add`, `override`, `validate`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use keinontolibrary_core::{Case, Engine, Error, Forms, Number, ParadigmRef};
use keinontolibrary_data::{build_engine, OverlayEntry};

/// Decline simple Finnish nouns.
#[derive(Parser, Debug)]
#[command(name = "keinontolibrary", version, about)]
struct Cli {
    /// Path to the packed lookup artifact.
    #[arg(
        long,
        env = "KEINONTO_ARTIFACT",
        default_value = "data/artifact/keinontolibrary.bin"
    )]
    artifact: PathBuf,
    /// Path to the overlay store (add/override).
    #[arg(long, env = "KEINONTO_OVERLAY", default_value = "data/overlay.jsonl")]
    overlay: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Decline a word into a single case/number.
    Decline {
        /// The baseform.
        word: String,
        #[arg(long)]
        number: Number,
        #[arg(long)]
        case: Case,
        /// Disambiguate by declension class.
        #[arg(long)]
        tn: Option<u8>,
        /// Disambiguate by homonym number.
        #[arg(long)]
        hn: Option<u8>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print the whole paradigm (all 30 slots) for a word.
    Paradigm {
        /// The baseform.
        word: String,
        #[arg(long)]
        tn: Option<u8>,
        #[arg(long)]
        hn: Option<u8>,
        #[arg(long)]
        json: bool,
    },
    /// Add forms for a slot to the overlay (creating the lemma if new).
    Add(EntryArgs),
    /// Override the forms for a slot in the overlay.
    Override(EntryArgs),
    /// Print artifact metadata and exit.
    Validate,
}

#[derive(clap::Args, Debug)]
struct EntryArgs {
    #[arg(long)]
    lemma: String,
    #[arg(long)]
    tn: u8,
    #[arg(long)]
    hn: Option<u8>,
    #[arg(long)]
    av: Option<char>,
    #[arg(long)]
    number: Number,
    #[arg(long)]
    case: Case,
    /// Comma-separated surface forms, primary first.
    #[arg(long, value_delimiter = ',', required = true)]
    forms: Vec<String>,
}

impl EntryArgs {
    fn into_entry(self) -> OverlayEntry {
        OverlayEntry {
            lemma: self.lemma,
            tn: self.tn,
            hn: self.hn,
            av: self.av,
            number: self.number,
            case: self.case,
            variants: self.forms,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let bundle = build_engine(&cli.artifact, &cli.overlay)
        .map_err(|e| anyhow::anyhow!("loading artifact {}: {e}", cli.artifact.display()))?;

    match cli.command {
        Command::Decline {
            word,
            number,
            case,
            tn,
            hn,
            json,
        } => {
            let result = decline(&bundle.engine, &word, number, case, tn, hn);
            print_forms_result(&word, number, case, &result, json);
        }
        Command::Paradigm { word, tn, hn, json } => {
            print_paradigm(&bundle.engine, &word, tn, hn, json);
        }
        Command::Add(args) | Command::Override(args) => {
            let entry = args.into_entry();
            bundle.overlay.append(&entry)?;
            println!(
                "overlay: {} {} {} = {:?}",
                entry.lemma, entry.number, entry.case, entry.variants
            );
        }
        Command::Validate => {
            let m = &bundle.meta;
            println!("version:       {}", m.version);
            println!("lemmas:        {}", m.n_lemmas);
            println!("forms:         {}", m.n_forms);
            println!("kotus source:  {}", m.kotus_source);
            println!("reference:     {}", m.reference_source);
        }
    }
    Ok(())
}

fn decline(
    engine: &Engine,
    word: &str,
    number: Number,
    case: Case,
    tn: Option<u8>,
    hn: Option<u8>,
) -> Result<Forms, Error> {
    match tn {
        Some(tn) => engine.decline_with(word, number, case, &ParadigmRef::new(hn, tn)),
        None => engine.decline(word, number, case),
    }
}

fn print_forms_result(
    word: &str,
    number: Number,
    case: Case,
    result: &Result<Forms, Error>,
    json: bool,
) {
    match result {
        Ok(forms) if json => println!("{}", serde_json::to_string(forms).unwrap_or_default()),
        Ok(forms) => {
            print!("{} ({number} {case}): {}", word, forms.variants.join(", "));
            if let Some(c) = forms.coincides_with {
                print!("  [= {c}]");
            }
            println!("  ({:?}, {:?})", forms.status, forms.source);
        }
        Err(e) => print_error(e),
    }
}

fn print_paradigm(engine: &Engine, word: &str, tn: Option<u8>, hn: Option<u8>, json: bool) {
    let result = match tn {
        Some(tn) => engine.paradigm_with(word, &ParadigmRef::new(hn, tn)),
        None => engine.paradigm(word),
    };
    match result {
        Ok(p) if json => println!("{}", serde_json::to_string(&p).unwrap_or_default()),
        Ok(p) => {
            println!("{} (tn={})", p.lemma, p.reference.tn);
            for (number, case, forms) in p.iter() {
                if forms.variants.is_empty() {
                    continue;
                }
                println!("  {number:<8} {case:<12} {}", forms.variants.join(", "));
            }
        }
        Err(e) => print_error(&e),
    }
}

fn print_error(e: &Error) {
    match e {
        Error::Ambiguous { lemma, paradigms } => {
            eprintln!("'{lemma}' is ambiguous; pass --tn (or --hn):");
            for p in paradigms {
                eprintln!("  {p}");
            }
        }
        other => eprintln!("{other}"),
    }
}
