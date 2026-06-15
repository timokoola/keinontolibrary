//! `keinontolibrary` CLI — `decline`, `paradigm`, `table`, `add`, `override`,
//! `validate`, `selftest`.

use std::path::PathBuf;
use std::process::ExitCode;

use std::fmt::Write as _;

use clap::{Parser, Subcommand, ValueEnum};
use keinontolibrary_core::{Case, Engine, Error, Forms, Generator, Number, Paradigm, ParadigmRef};
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
    /// Render full declension table(s) — case rows × singular/plural columns.
    Table {
        /// One or more baseforms.
        #[arg(required = true)]
        words: Vec<String>,
        /// Disambiguate by declension class (applies to each word).
        #[arg(long)]
        tn: Option<u8>,
        /// Disambiguate by homonym number.
        #[arg(long)]
        hn: Option<u8>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = TableFormat::Text)]
        format: TableFormat,
    },
    /// Add forms for a slot to the overlay (creating the lemma if new).
    Add(EntryArgs),
    /// Override the forms for a slot in the overlay.
    Override(EntryArgs),
    /// Print artifact metadata and exit.
    Validate,
    /// Self-test: decline a built-in golden set and verify the forms (no artifact
    /// needed). Exits 0 on success, 1 on any mismatch — for verifying an install.
    Selftest,
}

/// Output format for `table`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum TableFormat {
    /// Aligned plain-text grid.
    Text,
    /// GitHub-flavored Markdown table.
    Markdown,
    /// Comma-separated values (case, singular, plural).
    Csv,
    /// The paradigm as JSON.
    Json,
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
        // A lookup that found no answer (unknown/ambiguous/defective) is a distinct,
        // scriptable exit code from a setup error (bad artifact, I/O).
        Err(RunError::NotFound) => ExitCode::from(3),
        Err(RunError::Other(e)) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// CLI run outcome: `NotFound` for a query the engine could not answer (exit 3),
/// `Other` for setup/usage failures (exit 1).
enum RunError {
    NotFound,
    Other(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for RunError {
    fn from(e: E) -> Self {
        RunError::Other(e.into())
    }
}

fn run(cli: Cli) -> Result<(), RunError> {
    // selftest is artifact-free — it exercises the rule generator directly so it can
    // verify a fresh install before any data file exists.
    if matches!(cli.command, Command::Selftest) {
        return selftest();
    }

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
            // Scripts can rely on the exit code, not stderr parsing.
            if result.is_err() {
                return Err(RunError::NotFound);
            }
        }
        Command::Paradigm { word, tn, hn, json } => {
            if !print_paradigm(&bundle.engine, &word, tn, hn, json) {
                return Err(RunError::NotFound);
            }
        }
        Command::Table {
            words,
            tn,
            hn,
            format,
        } => {
            let mut any_missing = false;
            for (i, word) in words.iter().enumerate() {
                if i > 0 && format != TableFormat::Json {
                    println!();
                }
                let result = match tn {
                    Some(tn) => bundle.engine.paradigm_with(word, &ParadigmRef::new(hn, tn)),
                    None => bundle.engine.paradigm(word),
                };
                match result {
                    Ok(p) => println!("{}", render_table(&p, format)),
                    Err(e) => {
                        print_error(&e);
                        any_missing = true;
                    }
                }
            }
            if any_missing {
                return Err(RunError::NotFound);
            }
        }
        Command::Selftest => unreachable!("handled before engine build"),
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

/// Print a paradigm; returns `false` when the lemma could not be resolved (so the caller
/// can exit nonzero).
fn print_paradigm(engine: &Engine, word: &str, tn: Option<u8>, hn: Option<u8>, json: bool) -> bool {
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
        Err(e) => {
            print_error(&e);
            return false;
        }
    }
    true
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

/// One cell: the variants joined, or an em dash for a defective/empty slot.
fn cell(forms: &Forms) -> String {
    if forms.variants.is_empty() {
        "—".to_owned()
    } else {
        forms.variants.join(", ")
    }
}

/// Render a paradigm as a case-rows × singular/plural-columns table.
fn render_table(p: &Paradigm, format: TableFormat) -> String {
    if format == TableFormat::Json {
        return serde_json::to_string_pretty(p).unwrap_or_default();
    }
    // Gather rows once: (case name, singular cell, plural cell).
    let rows: Vec<(&str, String, String)> = Case::ALL
        .iter()
        .map(|&case| {
            (
                case.name(),
                cell(p.get(Number::Singular, case)),
                cell(p.get(Number::Plural, case)),
            )
        })
        .collect();
    let mut out = String::new();
    match format {
        TableFormat::Csv => {
            out.push_str("case,singular,plural\n");
            for (c, sg, pl) in &rows {
                let _ = writeln!(out, "{},{},{}", csv(c), csv(sg), csv(pl));
            }
        }
        TableFormat::Markdown => {
            let _ = writeln!(out, "**{}** (tn {})\n", p.lemma, p.reference.tn);
            out.push_str("| case | singular | plural |\n|---|---|---|\n");
            for (c, sg, pl) in &rows {
                let _ = writeln!(out, "| {c} | {sg} | {pl} |");
            }
        }
        TableFormat::Text => {
            // The plural column is last, so only the case and singular columns need
            // padding to align. Widths count chars (Finnish ä/ö are multibyte).
            let wc = rows
                .iter()
                .map(|(c, ..)| c.chars().count())
                .max()
                .unwrap_or(4)
                .max(4);
            let ws = rows
                .iter()
                .map(|(_, s, _)| s.chars().count())
                .max()
                .unwrap_or(8)
                .max(8);
            let _ = writeln!(out, "{} (tn {})", p.lemma, p.reference.tn);
            let _ = writeln!(out, "{:<wc$}  {:<ws$}  plural", "case", "singular");
            for (c, sg, pl) in &rows {
                let _ = writeln!(out, "{c:<wc$}  {sg:<ws$}  {pl}");
            }
        }
        TableFormat::Json => unreachable!(),
    }
    out.trim_end().to_owned()
}

/// CSV-quote a field if it contains a comma or quote.
fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// Built-in golden set for `selftest` — rule- and registry-generated forms (no corpus),
/// each Voikko-verified. `(lemma, tn, av, number, case, expected primary)`.
/// `(lemma, tn, av, number, case, expected primary form)`.
type GoldenCase = (&'static str, u8, Option<char>, Number, Case, &'static str);

const GOLDEN: &[GoldenCase] = &[
    ("talo", 1, None, Number::Singular, Case::Inessive, "talossa"),
    (
        "hevonen",
        38,
        None,
        Number::Plural,
        Case::Inessive,
        "hevosissa",
    ),
    (
        "aika",
        9,
        Some('D'),
        Number::Singular,
        Case::Genitive,
        "ajan",
    ),
    (
        "kuka",
        101,
        None,
        Number::Singular,
        Case::Accusative,
        "kenet",
    ),
    (
        "jokin",
        101,
        None,
        Number::Singular,
        Case::Inessive,
        "jossakin",
    ),
    (
        "vanhempi",
        16,
        None,
        Number::Singular,
        Case::Genitive,
        "vanhemman",
    ),
    (
        "sakset",
        7,
        None,
        Number::Plural,
        Case::Inessive,
        "saksissa",
    ),
    (
        "parfait",
        22,
        None,
        Number::Singular,
        Case::Partitive,
        "parfait'ta",
    ),
];

/// Run the golden set through the registry-aware rule engine (artifact-free) and report.
/// `RuleEngine::generate` consults the exception registry, so it covers the registry-only
/// pronouns (kuka/jokin, tn 101) and irregular overrides (aika's k:j) that the free
/// `keinontolibrary_rules::generate` cannot. The parfait citation needs a `ForeignCitation`.
fn selftest() -> Result<(), RunError> {
    use keinontolibrary_core::ForeignCitation;
    let engine = keinontolibrary_rules::RuleEngine::new();
    let mut failed = 0;
    for &(lemma, tn, av, number, case, expected) in GOLDEN {
        let reference = if lemma == "parfait" {
            ParadigmRef::new(None, tn).with_citation(Some(ForeignCitation {
                sep: '\'',
                front: false,
                echo: 'e',
            }))
        } else {
            ParadigmRef::new(None, tn).with_av(av)
        };
        let got = engine.generate(lemma, &reference, number, case);
        let primary = got
            .as_ref()
            .and_then(|f| f.variants.first())
            .map(String::as_str);
        let ok = primary == Some(expected);
        if !ok {
            failed += 1;
        }
        println!(
            "{} {lemma} {number} {case}: {} (want {expected})",
            if ok { "ok  " } else { "FAIL" },
            primary.unwrap_or("<none>"),
        );
    }
    if failed == 0 {
        println!("\nselftest: {} checks passed", GOLDEN.len());
        Ok(())
    } else {
        Err(RunError::Other(anyhow::anyhow!(
            "selftest: {failed}/{} checks failed",
            GOLDEN.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selftest_golden_set_passes() {
        // The same checks `keinontolibrary selftest` runs, as a unit test so a rule
        // regression is caught in CI without invoking the binary.
        assert!(selftest().is_ok());
    }
}
