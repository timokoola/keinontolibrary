//! Form generation for the high-frequency Kotus classes.
//!
//! For a `(lemma, tn, av)` we derive the singular vowel stem (strong/weak via gradation),
//! the plural `-i-` stem, and the class-specific partitive/illative/genitive-plural forms;
//! then assemble each slot with the uniform case endings and the grade table.
//!
//! Coverage is the pragmatic high-frequency set: classes 1-15, 17-20, 23, 24, 26-28, 32-34, 38-41, 43, 47, 48 (34 in all). Other classes return `None` (no generation; the lookup/overlay still answer).

use keinontolibrary_core::{Case, Number};

use crate::gradation::{grade, strengthen, weaken, Grade};
use crate::harmony::{aa, oo};

/// The derived stems and irregular slots for one word.
#[derive(Debug, Clone)]
struct Stems {
    /// Singular vowel stem, strong grade (partitive/illative/essive).
    sg_strong: String,
    /// Singular vowel stem, weak grade (genitive and the locatives).
    sg_weak: String,
    /// Plural `-i-` stem, strong grade.
    pl_strong: String,
    /// Plural `-i-` stem, weak grade.
    pl_weak: String,
    /// Singular partitive form(s).
    part_sg: Vec<String>,
    /// Singular illative form(s).
    illat_sg: Vec<String>,
    /// Plural genitive form(s).
    gen_pl: Vec<String>,
    /// Plural partitive form(s).
    part_pl: Vec<String>,
    /// Plural illative form(s).
    illat_pl: Vec<String>,
    /// Essive uses this stem (usually `sg_strong`); class 40 differs.
    essive_stem: String,
}

fn drop_last(s: &str) -> String {
    let mut c = s.chars();
    c.next_back();
    c.as_str().to_owned()
}

fn last_char(s: &str) -> Option<char> {
    s.chars().next_back()
}

fn ends_with_vowel(s: &str) -> bool {
    matches!(
        last_char(s),
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö')
    )
}

/// Form the plural `-i-` stem from a singular vowel stem, per class.
fn pluralize(stem: &str, tn: u8) -> String {
    let body = drop_last(stem);
    match last_char(stem) {
        Some('i') => format!("{body}ei"),
        // -a/-ä round to -o-/-ö- before the plural -i- (most classes)...
        Some('a') if tn != 10 => format!("{body}oi"),
        Some('ä') if tn != 10 => format!("{body}öi"),
        // ...or the final vowel just drops (type 10, and the -e stems).
        Some('e' | 'a' | 'ä') => format!("{body}i"),
        // -o/-u/-y/-ö (and anything else) just take -i.
        _ => format!("{stem}i"),
    }
}

/// Whether the character before the final `i` of a plural stem is a vowel (a diphthong
/// ending), which decides `-hin` vs `-in` illative and other plural endings.
fn ends_in_diphthong(pl: &str) -> bool {
    let chars: Vec<char> = pl.chars().collect();
    if chars.len() < 2 {
        return false;
    }
    matches!(
        chars[chars.len() - 2],
        'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö'
    )
}

/// Classes 1, 2, 5, 6, 9, 10, 12: the vowel stem is the lemma itself; gradation and the
/// plural `-i-` stem do the work.
fn analyze_vowel_stem(lemma: &str, tn: u8, av: Option<char>, a: &str) -> Stems {
    // Consonant-final tn5 loanwords (epsilon, nylon, stadion) inflect on an epenthetic -i-
    // stem (epsilon -> epsiloni-: epsilonin, epsiloneissa). The nominative stays the bare
    // lemma. Native tn5 words already end in -i, so this only affects the loanwords.
    let sg_strong = if tn == 5 && !ends_with_vowel(lemma) {
        format!("{lemma}i")
    } else {
        lemma.to_owned()
    };
    let sg_weak = weaken(&sg_strong, av);
    let pl_strong = pluralize(&sg_strong, tn);
    let pl_weak = pluralize(&sg_weak, tn);
    let last = last_char(&sg_strong).unwrap_or('a');
    let pl_body = drop_last(&pl_strong);
    let body = drop_last(&sg_strong);

    let part_sg = match tn {
        3 => vec![format!("{sg_strong}t{a}")], // valtiota (diphthong stem)
        _ => vec![format!("{sg_strong}{a}")],
    };
    let part_pl = match tn {
        1 | 5 | 9 => vec![format!("{pl_body}j{a}")], // valoja, ristejä, kaloja
        10 => vec![format!("{pl_strong}{a}")],       // koiria
        4 | 14 => vec![format!("{pl_weak}t{a}"), format!("{pl_body}j{a}")], // laatikoita, laatikkoja
        _ => vec![format!("{pl_strong}t{a}")], // 2,3,6,12,13: palveluita, valtioita, ...
    };
    let gen_pl = match tn {
        1 => vec![format!("{pl_body}jen")],
        9 => vec![format!("{pl_body}jen"), format!("{body}{a}in")], // kalojen, kalain
        2 => vec![format!("{pl_body}jen"), format!("{pl_strong}den")],
        5 | 6 => vec![format!("{sg_strong}en")],
        10 => vec![format!("{body}ien"), format!("{body}{a}in")], // koirien, koirain
        12 | 13 => vec![
            format!("{pl_strong}den"),
            format!("{pl_strong}tten"),
            format!("{body}{a}in"),
        ],
        3 => vec![format!("{pl_strong}den"), format!("{pl_strong}tten")], // valtioiden, valtioitten
        4 => vec![
            format!("{pl_body}jen"),
            format!("{pl_weak}den"),
            format!("{pl_weak}tten"),
        ],
        14 => vec![format!("{body}{a}in"), format!("{pl_weak}den")], // solakkain, solakoiden
        _ => vec![format!("{pl_strong}en")],
    };

    Stems {
        part_sg,
        illat_sg: vec![format!("{sg_strong}{last}n")],
        illat_pl: plural_illative(&pl_strong),
        essive_stem: sg_strong.clone(),
        sg_strong,
        sg_weak,
        pl_strong,
        pl_weak,
        gen_pl,
        part_pl,
    }
}

// A per-class dispatch: each arm is compact but there are many of them.
#[allow(clippy::too_many_lines)]
fn analyze(lemma: &str, tn: u8, av: Option<char>) -> Option<Stems> {
    let a = aa(lemma);
    if matches!(tn, 1 | 2 | 3 | 4 | 5 | 6 | 9 | 10 | 12 | 13 | 14) {
        return Some(analyze_vowel_stem(lemma, tn, av, a));
    }

    match tn {
        // ovi: the oblique stem replaces final -i with -e (ovi -> ove-, kurki -> kurje-).
        7 => {
            let sg_strong = format!("{}e", lemma.strip_suffix('i')?);
            let sg_weak = weaken(&sg_strong, av);
            let pl_strong = pluralize(&sg_strong, tn);
            let pl_weak = pluralize(&sg_weak, tn);
            Some(Stems {
                part_sg: vec![format!("{sg_strong}{a}")],
                illat_sg: vec![format!("{sg_strong}en")],
                gen_pl: vec![format!("{pl_strong}en")],
                part_pl: vec![format!("{pl_strong}{a}")],
                illat_pl: plural_illative(&pl_strong),
                essive_stem: sg_strong.clone(),
                sg_strong,
                sg_weak,
                pl_strong,
                pl_weak,
            })
        }
        // kytkin: -in -> -ime- oblique; partitive on the consonant stem (the lemma). Often
        // reverse-gradating on the root consonant (ahdin -> ahtimen), so strengthen the
        // base before appending -me-.
        33 => {
            let base = strengthen(lemma.strip_suffix('n')?, av);
            let sg = format!("{base}me");
            let pl = pluralize(&sg, tn);
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // vieras: -s drops and the preceding vowel lengthens (viera+a); long-vowel stem like
        // hame. Often reverse-gradating (rakas -> rakkaan).
        41 => {
            let dropped = lemma.strip_suffix('s')?;
            let last = last_char(dropped)?;
            let sg = strengthen(&format!("{dropped}{last}"), av);
            let pl = format!("{}i", drop_last(&sg)); // vieraa -> vierai
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{sg}seen")],
                gen_pl: vec![
                    format!("{pl}den"),
                    format!("{pl}tten"),
                    format!("{lemma}ten"),
                ],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}siin"), format!("{pl}hin")],
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // vapaa / filee: a vowel-final long-vowel stem (no -s to drop); partitive -ta,
        // illative -seen, plural drops one vowel before -i- (vapaa -> vapai-, filee -> filei-).
        17 | 20 => {
            let sg = lemma.to_owned();
            let pl = format!("{}i", drop_last(&sg));
            Some(Stems {
                part_sg: vec![format!("{sg}t{a}")],
                illat_sg: vec![format!("{sg}seen")],
                gen_pl: vec![format!("{pl}den"), format!("{pl}tten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}siin"), format!("{pl}hin")],
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // maa: monosyllabic long vowel; illative is -hVn (maa -> maahan).
        18 => {
            let sg = lemma.to_owned();
            let last = last_char(&sg)?;
            let pl = format!("{}i", drop_last(&sg));
            Some(Stems {
                part_sg: vec![format!("{sg}t{a}")],
                illat_sg: vec![format!("{sg}h{last}n")],
                gen_pl: vec![format!("{pl}den"), format!("{pl}tten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}hin")],
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // nalle: foreign -e words; the -e is kept before the plural -i- (nalle -> nallei-)
        // and the plural uses -j-.
        8 => {
            let sg = lemma.to_owned();
            let last = last_char(&sg)?;
            let pl = format!("{sg}i");
            let pl_body = drop_last(&pl);
            Some(Stems {
                part_sg: vec![format!("{sg}{a}")],
                illat_sg: vec![format!("{sg}{last}n")],
                gen_pl: vec![format!("{pl_body}jen")],
                part_pl: vec![format!("{pl_body}j{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // tiili/uni/pieni: -i -> -e- oblique, but partitive/genitive-plural use the
        // consonant stem (pieni -> pienen, pientä, pienten; tiili -> tiiltä; uni -> unta).
        23 | 24 | 26 => {
            let cons = lemma.strip_suffix('i')?.to_owned();
            let sg = format!("{cons}e");
            let sg_weak = weaken(&sg, av);
            let pl = pluralize(&sg, tn);
            let pl_weak = pluralize(&sg_weak, tn);
            Some(Stems {
                part_sg: vec![format!("{cons}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{cons}ten"), format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg,
                sg_weak,
                pl_strong: pl,
                pl_weak,
            })
        }
        // sisar: -r consonant stem; oblique adds -e- (sisar -> sisaren, sisarta). The
        // -tAr subtype is reverse-gradating (aallotar -> aallottaren), and the gradating
        // consonant sits before the final -r, so strengthen the body before re-adding it.
        32 => {
            let cons = last_char(lemma)?;
            let body = strengthen(&drop_last(lemma), av);
            let sg = format!("{body}{cons}e");
            let pl = pluralize(&sg, tn);
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{lemma}ten"), format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // korkea: -eA; partitive -a or -ta, plural drops the final -a before -i-
        // (korkea -> korkei-, korkeita).
        15 => {
            let sg = lemma.to_owned();
            let last = last_char(&sg).unwrap_or('a');
            let pl = format!("{}i", drop_last(&sg)); // korkea -> korkei
            let body = drop_last(&sg);
            Some(Stems {
                part_sg: vec![format!("{sg}{a}"), format!("{sg}t{a}")],
                illat_sg: vec![format!("{sg}{last}n")],
                gen_pl: vec![
                    format!("{pl}den"),
                    format!("{pl}tten"),
                    format!("{body}{a}in"),
                ],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // käsi: -si alternates with -te- (strong) / -de- (weak); partitive -ttA on the
        // bare root; plural keeps the -s- (käsi -> käden, kättä, käteen, käsiä).
        27 => {
            let base = lemma.strip_suffix("si")?;
            let strong = format!("{base}te");
            let weak = format!("{base}de");
            let pl = format!("{base}si");
            Some(Stems {
                part_sg: vec![format!("{base}tt{a}")],
                illat_sg: vec![format!("{strong}en")],
                gen_pl: vec![format!("{pl}en"), format!("{base}tten")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: strong.clone(),
                sg_strong: strong,
                sg_weak: weak,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // omena: 3-syllable -A with a dual plural (-i- and -oi-). Primary uses -i-; the
        // -oi- and -Ain forms are offered as variants.
        11 => {
            let sg = lemma.to_owned();
            let last = last_char(&sg).unwrap_or('a');
            let body = drop_last(&sg);
            let pl = format!("{body}i"); // omena -> omeni
            Some(Stems {
                part_sg: vec![format!("{sg}{a}")],
                illat_sg: vec![format!("{sg}{last}n")],
                gen_pl: vec![
                    format!("{body}{a}in"),
                    format!("{body}{}iden", oo(&sg)),
                    format!("{pl}en"),
                ],
                part_pl: vec![format!("{pl}{a}"), format!("{body}{}it{a}", oo(&sg))],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // kynsi: -si with assimilating weak stem (kynsi -> kynnen, varsi -> varren);
        // strong -te-, partitive -ttA, plural keeps -s-.
        28 => {
            let base = lemma.strip_suffix("si")?;
            let lastc = last_char(base)?;
            let strong = format!("{base}te");
            let weak = format!("{base}{lastc}e"); // kyn -> kynne, var -> varre
            let pl = format!("{base}si");
            Some(Stems {
                part_sg: vec![format!("{base}tt{a}")],
                illat_sg: vec![format!("{strong}en")],
                gen_pl: vec![format!("{pl}en"), format!("{base}tten")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: strong.clone(),
                sg_strong: strong,
                sg_weak: weak,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // onneton: -tOn -> -ttOmA- (onneton -> onnettoman, onnetonta, onnettomaan).
        34 => {
            let base = lemma
                .strip_suffix("ton")
                .or_else(|| lemma.strip_suffix("tön"))?;
            let o = oo(lemma);
            let stem = format!("{base}tt{o}m{a}"); // onnettoma / työttömä
            let pl = format!("{base}tt{o}mi");
            let last = last_char(&stem)?;
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{stem}{last}n")],
                gen_pl: vec![format!("{pl}en"), format!("{pl}den")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: stem.clone(),
                sg_strong: stem.clone(),
                sg_weak: stem,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // kuollut: -Ut participle -> -ee- (kuollut -> kuolleen, kuollutta, kuolleeseen).
        47 => {
            let base = lemma
                .strip_suffix("ut")
                .or_else(|| lemma.strip_suffix("yt"))?;
            let stem = format!("{base}ee");
            let pl = format!("{}i", drop_last(&stem)); // kuollee -> kuollei
            let pl_body = drop_last(&pl);
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{stem}seen")],
                gen_pl: vec![format!("{pl}den"), format!("{pl}tten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}hin"), format!("{pl_body}isiin")],
                essive_stem: stem.clone(),
                sg_strong: stem.clone(),
                sg_weak: stem,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // suo: monosyllabic diphthong; illative -hVn (suo -> suohon) and the plural reverses
        // the diphthong before -i- (suo -> soi-, tie -> tei-).
        19 => {
            let chars: Vec<char> = lemma.chars().collect();
            let last = *chars.last()?;
            let pl = if chars.len() >= 2 {
                let prefix: String = chars[..chars.len() - 2].iter().collect();
                format!("{prefix}{last}i")
            } else {
                format!("{lemma}i")
            };
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{lemma}h{last}n")],
                gen_pl: vec![format!("{pl}den"), format!("{pl}tten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}hin")],
                essive_stem: lemma.to_owned(),
                sg_strong: lemma.to_owned(),
                sg_weak: lemma.to_owned(),
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // ohut: -Ut -> -Ue- (ohut -> ohuen, ohutta, ohueen, ohuita).
        43 => {
            let base = lemma.strip_suffix('t')?;
            let sg = format!("{base}e");
            let pl = pluralize(&sg, tn);
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{pl}den"), format!("{pl}tten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg.clone(),
                sg_weak: sg,
                pl_strong: pl.clone(),
                pl_weak: pl,
            })
        }
        // tn49 askel/askele is intentionally NOT generated: it has free variation between a
        // short stem (askelen, askelessa) and a long -ee- stem (askeleen, askeleessa) that a
        // single generated stem can't reproduce, so the corpus lookup answers it instead.
        // nainen: nen -> se- (oblique), nais- (consonant stem).
        38 => {
            let base = lemma.strip_suffix("nen")?;
            let sg = format!("{base}se");
            let cons = format!("{base}s");
            let pl = pluralize(&sg, tn);
            Some(Stems {
                sg_strong: sg.clone(),
                sg_weak: sg.clone(),
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{cons}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{cons}ten"), format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg,
            })
        }
        // vastaus: s -> kse- (oblique), vastaus consonant stem.
        39 => {
            let base = lemma.strip_suffix('s')?;
            let sg = format!("{base}kse");
            let pl = pluralize(&sg, tn);
            Some(Stems {
                sg_strong: sg.clone(),
                sg_weak: sg.clone(),
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{sg}en")],
                gen_pl: vec![format!("{lemma}ten"), format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg,
            })
        }
        // kalleus (-uus/-yys): de- (gen), te- (illat/essive), -ksi- (plural), -tta (part).
        40 => {
            let base = lemma.strip_suffix('s')?;
            let de = format!("{base}de");
            let te = format!("{base}te");
            let pl = format!("{base}ksi");
            Some(Stems {
                sg_strong: de.clone(), // not used for partitive/illative (overridden below)
                sg_weak: de,
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{base}tt{a}")],
                illat_sg: vec![format!("{te}en")],
                gen_pl: vec![format!("{pl}en")],
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: te,
            })
        }
        // hame (-e): stem doubles to -ee; partitive -tta; illative -seen. Many -e words
        // have *reverse* gradation (nominative weak, oblique strong): aarre -> aarteen.
        48 => {
            let sg = strengthen(&format!("{lemma}e"), av);
            let pl = pluralize(&sg, tn);
            let pl_body = drop_last(&pl);
            Some(Stems {
                sg_strong: sg.clone(),
                sg_weak: sg.clone(),
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{lemma}tt{a}")],
                illat_sg: vec![format!("{sg}seen")],
                gen_pl: vec![format!("{pl_body}iden"), format!("{pl_body}itten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl: vec![format!("{pl}hin"), format!("{pl_body}isiin")],
                essive_stem: sg,
            })
        }
        _ => None,
    }
}

fn plural_illative(pl: &str) -> Vec<String> {
    if ends_in_diphthong(pl) {
        vec![format!("{pl}hin")]
    } else {
        vec![format!("{pl}in")]
    }
}

/// Generate the surface form(s) for one slot, or `None` if the class is unsupported.
#[must_use]
pub fn generate(
    lemma: &str,
    tn: u8,
    av: Option<char>,
    number: Number,
    case: Case,
) -> Option<Vec<String>> {
    let s = analyze(lemma, tn, av)?;
    let a = aa(lemma);
    let g = grade(number, case);
    let sg = if g == Grade::Strong {
        &s.sg_strong
    } else {
        &s.sg_weak
    };
    let pl = if g == Grade::Strong {
        &s.pl_strong
    } else {
        &s.pl_weak
    };

    let forms = match (number, case) {
        (Number::Singular, Case::Nominative) => vec![lemma.to_owned()],
        (Number::Singular, Case::Genitive | Case::Accusative) => vec![format!("{}n", s.sg_weak)],
        (Number::Singular, Case::Partitive) => s.part_sg.clone(),
        (Number::Singular, Case::Inessive) => vec![format!("{sg}ss{a}")],
        (Number::Singular, Case::Elative) => vec![format!("{sg}st{a}")],
        (Number::Singular, Case::Illative) => s.illat_sg.clone(),
        (Number::Singular, Case::Adessive) => vec![format!("{sg}ll{a}")],
        (Number::Singular, Case::Ablative) => vec![format!("{sg}lt{a}")],
        (Number::Singular, Case::Allative) => vec![format!("{sg}lle")],
        (Number::Singular, Case::Essive) => vec![format!("{}n{a}", s.essive_stem)],
        (Number::Singular, Case::Translative) => vec![format!("{sg}ksi")],
        (Number::Singular, Case::Abessive) => vec![format!("{sg}tt{a}")],
        (Number::Singular, Case::Comitative | Case::Instructive) => return None, // plural-only

        (Number::Plural, Case::Nominative | Case::Accusative) => vec![format!("{}t", s.sg_weak)],
        (Number::Plural, Case::Genitive) => s.gen_pl.clone(),
        (Number::Plural, Case::Partitive) => s.part_pl.clone(),
        (Number::Plural, Case::Inessive) => vec![format!("{pl}ss{a}")],
        (Number::Plural, Case::Elative) => vec![format!("{pl}st{a}")],
        (Number::Plural, Case::Illative) => s.illat_pl.clone(),
        (Number::Plural, Case::Adessive) => vec![format!("{pl}ll{a}")],
        (Number::Plural, Case::Ablative) => vec![format!("{pl}lt{a}")],
        (Number::Plural, Case::Allative) => vec![format!("{pl}lle")],
        (Number::Plural, Case::Essive) => vec![format!("{pl}n{a}")],
        (Number::Plural, Case::Translative) => vec![format!("{pl}ksi")],
        (Number::Plural, Case::Abessive) => vec![format!("{pl}tt{a}")],
        (Number::Plural, Case::Comitative) => {
            vec![
                format!("{}neen", s.pl_strong),
                format!("{}nens{a}", s.pl_strong),
            ]
        }
        (Number::Plural, Case::Instructive) => vec![format!("{}n", s.pl_weak)],
    };
    Some(forms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(lemma: &str, tn: u8, av: Option<char>, n: Number, c: Case) -> String {
        generate(lemma, tn, av, n, c).unwrap()[0].clone()
    }

    #[test]
    fn valo_class_1() {
        assert_eq!(
            one("valo", 1, None, Number::Singular, Case::Genitive),
            "valon"
        );
        assert_eq!(
            one("valo", 1, None, Number::Singular, Case::Inessive),
            "valossa"
        );
        assert_eq!(
            one("valo", 1, None, Number::Singular, Case::Illative),
            "valoon"
        );
        assert_eq!(
            one("valo", 1, None, Number::Plural, Case::Inessive),
            "valoissa"
        );
    }

    #[test]
    fn epsilon_consonant_final_tn5() {
        // Consonant-final tn5 loanword: epenthetic -i- stem, bare nominative.
        let g = |n, c| one("epsilon", 5, None, n, c);
        assert_eq!(g(Number::Singular, Case::Nominative), "epsilon");
        assert_eq!(g(Number::Singular, Case::Genitive), "epsilonin");
        assert_eq!(g(Number::Singular, Case::Inessive), "epsilonissa");
        assert_eq!(g(Number::Plural, Case::Nominative), "epsilonit");
        assert_eq!(g(Number::Plural, Case::Genitive), "epsilonien");
        assert_eq!(g(Number::Plural, Case::Partitive), "epsiloneja");
        assert_eq!(g(Number::Plural, Case::Inessive), "epsiloneissa");
        assert_eq!(g(Number::Plural, Case::Illative), "epsiloneihin");
        // Regression: vowel-final tn5 unchanged.
        assert_eq!(
            one("risti", 5, None, Number::Plural, Case::Inessive),
            "risteissä"
        );
        assert_eq!(
            one("viini", 5, None, Number::Singular, Case::Partitive),
            "viiniä"
        );
    }

    #[test]
    fn gradation_kortti_class_5() {
        assert_eq!(
            one("kortti", 5, Some('C'), Number::Singular, Case::Genitive),
            "kortin"
        );
        assert_eq!(
            one("kortti", 5, Some('C'), Number::Singular, Case::Partitive),
            "korttia"
        );
        assert_eq!(
            one("kortti", 5, Some('C'), Number::Singular, Case::Inessive),
            "kortissa"
        );
        assert_eq!(
            one("kortti", 5, Some('C'), Number::Singular, Case::Essive),
            "korttina"
        );
        assert_eq!(
            one("kortti", 5, Some('C'), Number::Plural, Case::Nominative),
            "kortit"
        );
    }

    #[test]
    fn kala_class_9_plural_o() {
        assert_eq!(
            one("kala", 9, None, Number::Plural, Case::Inessive),
            "kaloissa"
        );
        assert_eq!(
            one("kala", 9, None, Number::Singular, Case::Illative),
            "kalaan"
        );
    }

    #[test]
    fn nainen_class_38() {
        assert_eq!(
            one("nainen", 38, None, Number::Singular, Case::Genitive),
            "naisen"
        );
        assert_eq!(
            one("nainen", 38, None, Number::Singular, Case::Partitive),
            "naista"
        );
        assert_eq!(
            one("nainen", 38, None, Number::Singular, Case::Illative),
            "naiseen"
        );
        assert_eq!(
            one("nainen", 38, None, Number::Plural, Case::Inessive),
            "naisissa"
        );
    }

    #[test]
    fn hame_class_48() {
        assert_eq!(
            one("hame", 48, None, Number::Singular, Case::Genitive),
            "hameen"
        );
        assert_eq!(
            one("hame", 48, None, Number::Singular, Case::Partitive),
            "hametta"
        );
        assert_eq!(
            one("hame", 48, None, Number::Singular, Case::Illative),
            "hameeseen"
        );
    }

    #[test]
    fn valtio_class_3_partitive_ta() {
        assert_eq!(
            one("valtio", 3, None, Number::Singular, Case::Partitive),
            "valtiota"
        );
        assert_eq!(
            one("valtio", 3, None, Number::Singular, Case::Illative),
            "valtioon"
        );
        assert_eq!(
            one("valtio", 3, None, Number::Plural, Case::Partitive),
            "valtioita"
        );
    }

    #[test]
    fn laatikko_class_4_gradation() {
        assert_eq!(
            one("laatikko", 4, Some('A'), Number::Singular, Case::Genitive),
            "laatikon"
        );
        assert_eq!(
            one("laatikko", 4, Some('A'), Number::Singular, Case::Partitive),
            "laatikkoa"
        );
        assert_eq!(
            one("laatikko", 4, Some('A'), Number::Plural, Case::Nominative),
            "laatikot"
        );
    }

    #[test]
    fn ovi_class_7_i_to_e() {
        assert_eq!(
            one("ovi", 7, None, Number::Singular, Case::Genitive),
            "oven"
        );
        assert_eq!(
            one("ovi", 7, None, Number::Singular, Case::Partitive),
            "ovea"
        );
        assert_eq!(
            one("ovi", 7, None, Number::Singular, Case::Illative),
            "oveen"
        );
        assert_eq!(
            one("ovi", 7, None, Number::Plural, Case::Inessive),
            "ovissa"
        );
        // direct gradation: kurki -> kurjen
        assert_eq!(
            one("kurki", 7, Some('L'), Number::Singular, Case::Genitive),
            "kurjen"
        );
        assert_eq!(
            one("kurki", 7, Some('L'), Number::Singular, Case::Partitive),
            "kurkea"
        );
    }

    #[test]
    fn kytkin_class_33() {
        assert_eq!(
            one("kytkin", 33, None, Number::Singular, Case::Genitive),
            "kytkimen"
        );
        assert_eq!(
            one("kytkin", 33, None, Number::Singular, Case::Partitive),
            "kytkintä"
        );
        assert_eq!(
            one("kytkin", 33, None, Number::Plural, Case::Partitive),
            "kytkimiä"
        );
    }

    #[test]
    fn vieras_class_41() {
        assert_eq!(
            one("vieras", 41, None, Number::Singular, Case::Genitive),
            "vieraan"
        );
        assert_eq!(
            one("vieras", 41, None, Number::Singular, Case::Partitive),
            "vierasta"
        );
        assert_eq!(
            one("vieras", 41, None, Number::Singular, Case::Illative),
            "vieraaseen"
        );
        assert_eq!(
            one("vieras", 41, None, Number::Plural, Case::Partitive),
            "vieraita"
        );
    }

    #[test]
    fn vapaa_18_maa_long_vowel() {
        assert_eq!(
            one("vapaa", 17, None, Number::Singular, Case::Partitive),
            "vapaata"
        );
        assert_eq!(
            one("vapaa", 17, None, Number::Singular, Case::Illative),
            "vapaaseen"
        );
        assert_eq!(
            one("maa", 18, None, Number::Singular, Case::Illative),
            "maahan"
        );
        assert_eq!(
            one("maa", 18, None, Number::Plural, Case::Partitive),
            "maita"
        );
    }

    #[test]
    fn pieni_26_and_sisar_32() {
        assert_eq!(
            one("pieni", 26, None, Number::Singular, Case::Genitive),
            "pienen"
        );
        assert_eq!(
            one("pieni", 26, None, Number::Singular, Case::Partitive),
            "pientä"
        );
        assert_eq!(
            one("sisar", 32, None, Number::Singular, Case::Genitive),
            "sisaren"
        );
        assert_eq!(
            one("sisar", 32, None, Number::Singular, Case::Partitive),
            "sisarta"
        );
        // -tAr reverse gradation: aallotar -> aallottaren
        assert_eq!(
            one("aallotar", 32, Some('C'), Number::Singular, Case::Genitive),
            "aallottaren"
        );
    }

    #[test]
    fn nalle_8_tiili_23_uni_24() {
        assert_eq!(
            one("nalle", 8, None, Number::Singular, Case::Partitive),
            "nallea"
        );
        assert_eq!(
            one("nalle", 8, None, Number::Plural, Case::Genitive),
            "nallejen"
        );
        assert_eq!(
            one("tiili", 23, None, Number::Singular, Case::Partitive),
            "tiiltä"
        );
        assert_eq!(
            one("uni", 24, None, Number::Singular, Case::Genitive),
            "unen"
        );
        assert_eq!(
            one("uni", 24, None, Number::Singular, Case::Partitive),
            "unta"
        );
    }

    #[test]
    fn korkea_15_and_kasi_27() {
        assert_eq!(
            one("korkea", 15, None, Number::Singular, Case::Genitive),
            "korkean"
        );
        assert_eq!(
            one("korkea", 15, None, Number::Plural, Case::Partitive),
            "korkeita"
        );
        // käsi: si -> de (weak) / te (strong), partitive kättä
        assert_eq!(
            one("käsi", 27, None, Number::Singular, Case::Genitive),
            "käden"
        );
        assert_eq!(
            one("käsi", 27, None, Number::Singular, Case::Partitive),
            "kättä"
        );
        assert_eq!(
            one("käsi", 27, None, Number::Singular, Case::Illative),
            "käteen"
        );
        assert_eq!(
            one("käsi", 27, None, Number::Singular, Case::Essive),
            "kätenä"
        );
    }

    #[test]
    fn kynsi_28_onneton_34_kuollut_47_suo_19() {
        assert_eq!(
            one("kynsi", 28, None, Number::Singular, Case::Genitive),
            "kynnen"
        );
        assert_eq!(
            one("kynsi", 28, None, Number::Singular, Case::Partitive),
            "kynttä"
        );
        assert_eq!(
            one("onneton", 34, None, Number::Singular, Case::Genitive),
            "onnettoman"
        );
        assert_eq!(
            one("onneton", 34, None, Number::Singular, Case::Partitive),
            "onnetonta"
        );
        assert_eq!(
            one("kuollut", 47, None, Number::Singular, Case::Genitive),
            "kuolleen"
        );
        assert_eq!(
            one("kuollut", 47, None, Number::Plural, Case::Partitive),
            "kuolleita"
        );
        assert_eq!(
            one("suo", 19, None, Number::Singular, Case::Illative),
            "suohon"
        );
        assert_eq!(
            one("suo", 19, None, Number::Plural, Case::Genitive),
            "soiden"
        );
        assert_eq!(
            one("ohut", 43, None, Number::Singular, Case::Genitive),
            "ohuen"
        );
        assert_eq!(
            one("ohut", 43, None, Number::Plural, Case::Partitive),
            "ohuita"
        );
    }

    #[test]
    fn unsupported_class_returns_none() {
        // tn44 (kevät) is still unsupported.
        assert!(generate("kevät", 44, None, Number::Singular, Case::Genitive).is_none());
    }
}
