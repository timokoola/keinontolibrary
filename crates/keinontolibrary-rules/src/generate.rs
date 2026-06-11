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
        // Lemma-final -e in the vowel-stem classes (tn1 tempe, tn2 anime/college, tn3
        // aaloe — all loanwords) keeps the vowel before the plural -i-, like -o does:
        // animeissa, aaloeita (Voikko-verified; dropping gives *animissa, *aaloita).
        // Oblique stems in -e from other classes (tn7 ove-, tn33 kytkime-) still drop it.
        Some('e') if matches!(tn, 1..=3) => format!("{stem}i"),
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
    // Consonant-final tn5/tn6 loanwords (epsilon, nylon, stadion; agar, tomaatti-less -r/-n
    // borrowings) inflect on an epenthetic -i- stem (epsilon -> epsiloni-: epsilonin,
    // epsiloneissa; agar -> agari-: agarin, agareissa). The nominative stays the bare lemma;
    // native tn5/tn6 words already end in -i, so this only affects the loanwords.
    let sg_strong = if matches!(tn, 5 | 6) && !ends_with_vowel(lemma) {
        format!("{lemma}i")
    } else {
        lemma.to_owned()
    };
    let sg_weak = weaken(&sg_strong, av);
    let pl_strong = pluralize(&sg_strong, tn);
    // The k-elision apostrophe is surface orthography between IDENTICAL vowels only:
    // singular vaa'an but plural vaaoissa (rounding makes the vowels differ), singular
    // koon (long vowel) but plural ko'oissa (the pair re-opens before -i-). All
    // Voikko-verified. So: drop any apostrophe before forming the plural stem, then
    // re-add it only when an identical pair survives before the plural -i-.
    let pl_weak = weak_plural_stem(&sg_strong, &sg_weak, tn);
    let last = last_char(&sg_strong).unwrap_or('a');
    let pl_body = drop_last(&pl_strong);
    let body = drop_last(&sg_strong);

    let part_sg = match tn {
        3 => vec![format!("{sg_strong}t{a}")], // valtiota (diphthong stem)
        _ => vec![format!("{sg_strong}{a}")],
    };
    // tn2 -e loanwords take the light j-endings on the kept -e- (animeja, animejen;
    // Voikko rejects *animeita and *animeiden), unlike the native tn2 -o/-u words.
    let e_final = last_char(&sg_strong) == Some('e');
    let part_pl = match tn {
        1 | 5 | 9 => vec![format!("{pl_body}j{a}")], // valoja, ristejä, kaloja
        2 if e_final => vec![format!("{pl_body}j{a}")], // animeja, collegeja
        10 => vec![format!("{pl_strong}{a}")],       // koiria
        4 | 14 => vec![format!("{pl_weak}t{a}"), format!("{pl_body}j{a}")], // laatikoita, laatikkoja
        _ => vec![format!("{pl_strong}t{a}")], // 2,3,6,12,13: palveluita, valtioita, ...
    };
    let gen_pl = match tn {
        1 => vec![format!("{pl_body}jen")],
        9 => vec![format!("{pl_body}jen"), format!("{body}{a}in")], // kalojen, kalain
        2 if e_final => vec![format!("{pl_body}jen")],              // animejen, collegejen
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

/// The weak plural `-i-` stem, with k-elision apostrophe orthography. The apostrophe is
/// surface spelling between IDENTICAL vowels only: singular `vaa'an` but plural
/// `vaaoissa` (rounding differentiates), singular `koon` (long vowel) but plural
/// `ko'oissa`; the pair can also be formed WITH the plural -i- (`hiki → hi'issä`,
/// `ikä → i'issä`) — all Voikko-verified. Any apostrophe in the weak singular stem is
/// dropped before pluralizing and re-added only where an identical pair survives.
fn weak_plural_stem(sg_strong: &str, sg_weak: &str, tn: u8) -> String {
    let elided = sg_weak.chars().count() + 1 == sg_strong.chars().count();
    let mut pl_weak = pluralize(&sg_weak.replace('\'', ""), tn);
    if elided {
        let cs: Vec<char> = pl_weak.chars().collect();
        let is_v = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö');
        if let [.., x, y, 'i'] = cs[..] {
            if x == y && is_v(x) {
                let body: String = cs[..cs.len() - 2].iter().collect();
                pl_weak = format!("{body}'{y}i");
            } else if y == 'i' {
                let body: String = cs[..cs.len() - 1].iter().collect();
                pl_weak = format!("{body}'i");
            }
        } else if let [.., y, 'i'] = cs[..] {
            if y == 'i' {
                let body: String = cs[..cs.len() - 1].iter().collect();
                pl_weak = format!("{body}'i");
            }
        }
    }
    pl_weak
}

/// The harmonic `a`/`ä` for endings, honoring a lexical override (see [`generate`]).
fn harmony_a(front: Option<bool>, lemma: &str) -> &'static str {
    match front {
        Some(true) => "ä",
        Some(false) => "a",
        None => aa(lemma),
    }
}

/// The harmonic `o`/`ö` (tn34 -tOn stems), honoring a lexical override.
fn harmony_o(front: Option<bool>, lemma: &str) -> char {
    match front {
        Some(true) => 'ö',
        Some(false) => 'o',
        None => oo(lemma),
    }
}

// A per-class dispatch: each arm is compact but there are many of them.
#[allow(clippy::too_many_lines)]
fn analyze(lemma: &str, tn: u8, av: Option<char>, front: Option<bool>) -> Option<Stems> {
    // Letter-words with no vowel (cd, tv, dvd, www) decline on the pronounced letter
    // names with a colon (cd:n, tv:ssä) — the vowel-stem machinery can only produce
    // non-words (*tvn) for them, so decline generation (the lookup still answers).
    if !lemma
        .chars()
        .any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ä' | 'ö'))
    {
        return None;
    }
    // A consonant-final lemma in the long-vowel classes (17–20: vapaa/maa/suo/filee) is a
    // letter-word (adhd, tn18) declining on pronounced letter names with a colon — the
    // written stem can only produce non-words (*adhdssa), so leave it to the lookup.
    if matches!(tn, 17..=20) && !ends_with_vowel(lemma) {
        return None;
    }
    // Letter-name citations (sora-r, suhu-s) decline on the letter name with a colon —
    // also the lookup's job.
    if lemma
        .rsplit('-')
        .next()
        .is_some_and(|t| t.chars().count() == 1)
    {
        return None;
    }
    // Citation forms that hide the vowel stem: the cardinal numerals kahdeksan/
    // seitsemän/yhdeksän (tn10) inflect on the n-less stem (kahdeksassa, kahdeksia),
    // and kymmenen (tn32) on kymmen- (kymmentä, kymmenessä, kymmenten) — both
    // Voikko-verified. The nominative keeps the full citation (handled in `generate`).
    let lemma = if tn == 10
        && (lemma.ends_with("kahdeksan")
            || lemma.ends_with("seitsemän")
            || lemma.ends_with("yhdeksän"))
    {
        &lemma[..lemma.len() - 1]
    } else if tn == 32 && lemma.ends_with("kymmenen") {
        &lemma[..lemma.len() - 2]
    } else {
        lemma
    };
    let a = harmony_a(front, lemma);
    if matches!(tn, 1 | 2 | 3 | 4 | 5 | 6 | 9 | 10 | 12 | 13 | 14) {
        return Some(analyze_vowel_stem(lemma, tn, av, a));
    }

    match tn {
        // ovi: the oblique stem replaces final -i with -e (ovi -> ove-, kurki -> kurje-).
        7 => {
            let sg_strong = format!("{}e", lemma.strip_suffix('i')?);
            let sg_weak = weaken(&sg_strong, av);
            let pl_strong = pluralize(&sg_strong, tn);
            let pl_weak = weak_plural_stem(&sg_strong, &sg_weak, tn);
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
            // nukke gradates like any vowel stem: nuken, nukeissa (Voikko-verified;
            // the strong grade stays in the j-forms nukkeja/nukkejen).
            let sg = lemma.to_owned();
            let sg_weak = weaken(&sg, av);
            let last = last_char(&sg)?;
            let pl = format!("{sg}i");
            let pl_weak = format!("{sg_weak}i");
            let pl_body = drop_last(&pl);
            Some(Stems {
                part_sg: vec![format!("{sg}{a}")],
                illat_sg: vec![format!("{sg}{last}n")],
                gen_pl: vec![format!("{pl_body}jen")],
                part_pl: vec![format!("{pl_body}j{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: sg.clone(),
                sg_strong: sg,
                sg_weak,
                pl_strong: pl,
                pl_weak,
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
            let o = harmony_o(front, lemma);
            // The -tOn suffix doubles its t only after a vowel: onneton -> onnettoman,
            // but alaston -> alastoman (Voikko rejects *alasttoman).
            let tt = if ends_with_vowel(base) { "tt" } else { "t" };
            let stem = format!("{base}{tt}{o}m{a}"); // onnettoma / työttömä / alastoma
            let pl = format!("{base}{tt}{o}mi");
            let last = last_char(&stem)?;
            Some(Stems {
                part_sg: vec![format!("{lemma}t{a}")],
                illat_sg: vec![format!("{stem}{last}n")],
                // onnettomien + the consonant-stem onnetonten (Voikko rejects *-miden).
                gen_pl: vec![format!("{pl}en"), format!("{lemma}ten")],
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
            // Plurale tantum citations (alkeet, harteet, kamppeet) already carry the
            // long oblique stem with its grade: strip the -t (alkeet -> alkee-).
            // Treating them as hame-singulars appended a spurious -e (*alkeetihin).
            let stripped = lemma.strip_suffix('t').filter(|s| s.ends_with("ee"));
            let tantum = stripped.is_some();
            let sg = match stripped {
                Some(stem) => stem.to_owned(),
                None => strengthen(&format!("{lemma}e"), av),
            };
            let pl = pluralize(&sg, tn);
            let pl_body = drop_last(&pl);
            // -isiin is the attested primary plural illative (hameisiin, alkeisiin);
            // -ihin is a valid secondary for the singular-citation words only (hameihin
            // — Voikko rejects *alkeihin for the plurale tantum set).
            let illat_pl = if tantum {
                vec![format!("{pl_body}isiin")]
            } else {
                vec![format!("{pl_body}isiin"), format!("{pl}hin")]
            };
            Some(Stems {
                sg_strong: sg.clone(),
                sg_weak: sg.clone(),
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{lemma}tt{a}")],
                illat_sg: vec![format!("{sg}seen")],
                gen_pl: vec![format!("{pl_body}iden"), format!("{pl_body}itten")],
                part_pl: vec![format!("{pl}t{a}")],
                illat_pl,
                essive_stem: sg,
            })
        }
        // Ordinals (kolmas, kymmenes, neljäs, ...): the -s nominative inflects on a -nne-
        // (weak) / -nte- (strong) oblique stem with a -nsi- plural; partitive is -tta. So
        // kolmas -> kolmannen (gen), kolmatta (part), kolmanteen (illat), kolmantena (ess),
        // kolmansissa (pl ine). Also covers the pronominal ordinal `mones`.
        45 => {
            // Compound ordinals (kahdeskymmenes) inflect BOTH parts
            // (kahdennenkymmenennen) — out of v1 scope (see README); head-only
            // generation produces non-words, so decline and leave it to the lookup.
            if lemma.ends_with("kymmenes") && lemma.chars().count() > 8 {
                return None;
            }
            let base = lemma.strip_suffix('s')?;
            let nne = format!("{base}nne");
            let nte = format!("{base}nte");
            let pl = format!("{base}nsi");
            Some(Stems {
                sg_strong: nte.clone(),
                sg_weak: nne,
                pl_strong: pl.clone(),
                pl_weak: pl.clone(),
                part_sg: vec![format!("{base}tt{a}")],
                illat_sg: vec![format!("{nte}en")],
                gen_pl: vec![format!("{pl}en")], // kolmansien (ordinals take -nsien, not -sten)
                part_pl: vec![format!("{pl}{a}")],
                illat_pl: plural_illative(&pl),
                essive_stem: nte,
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
///
/// `adjective` marks modifier-only lemmas (adjectives/numerals), which take the bare
/// `-ine` plural comitative instead of the noun citation `-ineen`/`-inensA`.
/// `front` overrides vowel harmony (`Some(true)` = front endings) for compounds whose
/// final component flips it (antigeenissä) — segmentation is lexical knowledge the
/// generator cannot derive from the spelling alone.
#[must_use]
pub fn generate(
    lemma: &str,
    tn: u8,
    av: Option<char>,
    adjective: bool,
    front: Option<bool>,
    number: Number,
    case: Case,
) -> Option<Vec<String>> {
    // tn48 plurale tantum citations (alkeet, harteet) have no singular at all —
    // Voikko rejects *alkeen; the corpus marks the slots missing.
    if tn == 48 && number == Number::Singular && lemma.ends_with("eet") {
        return None;
    }
    let s = analyze(lemma, tn, av, front)?;
    let a = harmony_a(front, lemma);
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
            if adjective {
                // Modifiers agree with their head bare: punaisine (poskineen); only the
                // head noun carries the possessive (Voikko rejects *punaisineen).
                vec![format!("{}ne", s.pl_strong)]
            } else {
                vec![
                    format!("{}neen", s.pl_strong),
                    format!("{}nens{a}", s.pl_strong),
                ]
            }
        }
        (Number::Plural, Case::Instructive) => vec![format!("{}n", s.pl_weak)],
    };
    Some(forms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(lemma: &str, tn: u8, av: Option<char>, n: Number, c: Case) -> String {
        generate(lemma, tn, av, false, None, n, c).unwrap()[0].clone()
    }

    // Vowel-stem-class lemmas ending in -e (tn1 tempe, tn2 anime/college, tn3 aaloe — all
    // loanwords) keep the stem vowel before the plural -i- (Voikko-verified: aaloeita,
    // animeissa; *aaloita/*animissa are non-words). tn2 -e additionally takes the light
    // j-endings: animeja/animejen, not *animeita/*animeiden. Found by the QA loop.
    #[test]
    fn vowel_stem_final_e_keeps_stem_vowel_in_plural() {
        let g = |n, c| one("aaloe", 3, None, n, c);
        assert_eq!(g(Number::Plural, Case::Partitive), "aaloeita");
        assert_eq!(g(Number::Plural, Case::Inessive), "aaloeissa");
        assert_eq!(g(Number::Plural, Case::Genitive), "aaloeiden");
        assert_eq!(
            one("oboe", 3, None, Number::Plural, Case::Inessive),
            "oboeissa"
        );
        let anime = |n, c| generate("anime", 2, None, false, None, n, c).unwrap();
        assert_eq!(
            anime(Number::Plural, Case::Inessive),
            vec!["animeissa".to_owned()]
        );
        assert_eq!(anime(Number::Plural, Case::Partitive), vec!["animeja"]);
        assert_eq!(anime(Number::Plural, Case::Genitive), vec!["animejen"]);
        assert_eq!(
            one("tempe", 1, None, Number::Plural, Case::Inessive),
            "tempeissä"
        );
        // The plain tn2/tn3 vowel stems are unaffected.
        assert_eq!(
            one("valtio", 3, None, Number::Plural, Case::Inessive),
            "valtioissa"
        );
        assert_eq!(
            one("palvelu", 2, None, Number::Plural, Case::Partitive),
            "palveluita"
        );
    }

    // tn48 plurale tantum citations (alkeet) already carry the long stem: alkeisiin,
    // alkeilla — not the hame-template garbage *alkeetihin. Voikko-verified. Found by
    // the QA loop.
    #[test]
    fn tn48_plurale_tantum_strips_citation_t() {
        let g = |c| one("alkeet", 48, None, Number::Plural, c);
        assert_eq!(g(Case::Illative), "alkeisiin");
        assert_eq!(g(Case::Adessive), "alkeilla");
        assert_eq!(g(Case::Partitive), "alkeita");
        // The normal hame template is unaffected.
        assert_eq!(
            one("hame", 48, None, Number::Plural, Case::Illative),
            "hameisiin"
        );
    }

    // Letter-words decline on pronounced letter names with a colon (cd:n, adhd:n); the
    // written stem can only produce non-words (*tvn, *adhdssa), so the generator
    // declines: no vowels at all, or consonant-final in the long-vowel classes 17–20
    // (adhd is Kotus tn18 despite the spelled 'a'). Found by the QA loop.
    #[test]
    fn letter_words_are_not_generated() {
        assert!(generate(
            "tv",
            18,
            None,
            false,
            None,
            Number::Singular,
            Case::Genitive
        )
        .is_none());
        assert!(generate("dvd", 18, None, false, None, Number::Plural, Case::Inessive).is_none());
        assert!(generate(
            "adhd",
            18,
            None,
            false,
            None,
            Number::Singular,
            Case::Inessive
        )
        .is_none());
        // Genuine tn18 vowel-final words still generate.
        assert_eq!(
            one("maa", 18, None, Number::Singular, Case::Inessive),
            "maassa"
        );
    }

    // tn34 genitive plural: onnettomien + the consonant-stem onnetonten; the earlier
    // *-miden secondary was a non-word (Voikko-verified). Found by the QA loop.
    #[test]
    fn tn34_genitive_plural_variants() {
        let forms = generate(
            "onneton",
            34,
            Some('C'),
            false,
            None,
            Number::Plural,
            Case::Genitive,
        )
        .unwrap();
        assert_eq!(forms, vec!["onnettomien", "onnetonten"]);
        assert_eq!(
            generate(
                "työtön",
                34,
                Some('C'),
                true,
                None,
                Number::Plural,
                Case::Genitive
            )
            .unwrap(),
            vec!["työttömien", "työtönten"]
        );
    }

    // k-elision orthography (all Voikko-verified): apostrophe between identical vowels
    // when a vowel precedes the gap (vaaka -> vaa'an — but vaaoissa in the plural), long vowel after a
    // consonant (koko -> koon) — but the plural -i- stem re-opens the boundary
    // (ko'oissa, not *kooissa). Found by the QA loop.
    #[test]
    fn k_elision_apostrophe_orthography() {
        let g = |l, tn, n, c| one(l, tn, Some('D'), n, c);
        assert_eq!(g("vaaka", 9, Number::Singular, Case::Genitive), "vaa'an");
        assert_eq!(g("vaaka", 9, Number::Plural, Case::Inessive), "vaaoissa");
        assert_eq!(g("koko", 1, Number::Singular, Case::Genitive), "koon");
        assert_eq!(g("koko", 1, Number::Plural, Case::Inessive), "ko'oissa");
        assert_eq!(g("rako", 1, Number::Singular, Case::Genitive), "raon");
        // Elision between different vowels stays plain (reikä -> reiän, haku -> hauissa),
        // and so do stems whose final vowel rounds before the plural -i- (haka -> haoissa).
        assert_eq!(g("reikä", 9, Number::Singular, Case::Genitive), "reiän");
        assert_eq!(g("haku", 1, Number::Plural, Case::Inessive), "hauissa");
        assert_eq!(g("haka", 9, Number::Plural, Case::Inessive), "haoissa");
        // The pair can also be formed WITH the plural -i- (hiki -> hi'issä, ikä -> i'issä).
        assert_eq!(g("hiki", 7, Number::Plural, Case::Inessive), "hi'issä");
        assert_eq!(g("ikä", 10, Number::Plural, Case::Inessive), "i'issä");
        // mäki's pair differs (ä+i) -> plain mäissä.
        assert_eq!(g("mäki", 7, Number::Plural, Case::Inessive), "mäissä");
        // The apostrophe is singular-only for the vaaka type: rounding differentiates
        // the vowels in the plural (vaa'an but vaaoissa — Voikko-verified).
        assert_eq!(g("raaka", 9, Number::Plural, Case::Inessive), "raaoissa");
        assert_eq!(g("vaaka", 9, Number::Plural, Case::Adessive), "vaaoilla");
    }

    // Citation forms hiding the vowel stem (Voikko-verified): the -n cardinals and
    // kymmenen. Compound ordinals inflect both parts — out of scope, so no generation.
    // Found by the QA loop.
    #[test]
    fn numeral_citation_stems() {
        assert_eq!(
            one("kahdeksan", 10, None, Number::Singular, Case::Inessive),
            "kahdeksassa"
        );
        assert_eq!(
            one("kahdeksan", 10, None, Number::Plural, Case::Inessive),
            "kahdeksissa"
        );
        assert_eq!(
            one("kahdeksan", 10, None, Number::Singular, Case::Nominative),
            "kahdeksan"
        );
        assert_eq!(
            one("kymmenen", 32, None, Number::Singular, Case::Partitive),
            "kymmentä"
        );
        assert_eq!(
            one("kymmenen", 32, None, Number::Singular, Case::Inessive),
            "kymmenessä"
        );
        assert!(generate(
            "kahdeskymmenes",
            45,
            None,
            true,
            None,
            Number::Singular,
            Case::Genitive
        )
        .is_none());
    }

    // Reverse D-gradation is k-insertion before the final long vowel (Voikko-verified:
    // kokeen, kiukaassa, ikeen, okaita; previously only the registry knew aie). Found
    // by the QA loop.
    #[test]
    fn reverse_d_gradation_inserts_k() {
        let g = |l, tn, n, c| one(l, tn, Some('D'), n, c);
        assert_eq!(g("koe", 48, Number::Singular, Case::Genitive), "kokeen");
        assert_eq!(g("jae", 48, Number::Plural, Case::Inessive), "jakeissa");
        assert_eq!(g("aie", 48, Number::Singular, Case::Genitive), "aikeen");
        assert_eq!(
            g("kiuas", 41, Number::Singular, Case::Inessive),
            "kiukaassa"
        );
        assert_eq!(g("ies", 41, Number::Singular, Case::Genitive), "ikeen");
        assert_eq!(g("oas", 41, Number::Plural, Case::Partitive), "okaita");
        // Nominatives and consonant-stem forms stay on the citation.
        assert_eq!(g("koe", 48, Number::Singular, Case::Nominative), "koe");
        assert_eq!(g("koe", 48, Number::Singular, Case::Partitive), "koetta");
        assert_eq!(g("kiuas", 41, Number::Singular, Case::Partitive), "kiuasta");
    }

    // The -tOn suffix doubles its t only after a vowel (Voikko rejects *alasttoman).
    #[test]
    fn tn34_consonant_base_single_t() {
        assert_eq!(
            one("alaston", 34, None, Number::Singular, Case::Genitive),
            "alastoman"
        );
        assert_eq!(
            one("onneton", 34, Some('C'), Number::Singular, Case::Genitive),
            "onnettoman"
        );
    }

    // A lexical harmony override (minted from Voikko's segmentation at ingest) flips the
    // endings: antigeeni is anti+geeni, so front despite the spelled 'a' — while simplex
    // tyranni stays back by the default last-strong-vowel rule. Found by the QA loop.
    #[test]
    fn front_harmony_override_flips_endings() {
        let f = |front, n, c| generate("antigeeni", 5, None, false, front, n, c).unwrap();
        assert_eq!(
            f(Some(true), Number::Singular, Case::Inessive),
            vec!["antigeenissä"]
        );
        assert_eq!(
            f(Some(true), Number::Plural, Case::Partitive),
            vec!["antigeenejä"]
        );
        // Without the override the spelled 'a' wins (the old, wrong behavior for
        // compounds — kept as the default for simplex words like tyranni).
        assert_eq!(
            f(None, Number::Singular, Case::Inessive),
            vec!["antigeenissa"]
        );
    }

    // Modifiers (adjectives/numerals) take the bare -ine plural comitative; only head
    // nouns carry the possessive citation -ineen/-inensA. Voikko-verified: punaisine,
    // nopeine; *punaisineen rejected. Found by the QA loop.
    #[test]
    fn adjective_comitative_is_bare_ine() {
        let adj = |l, tn, n, c| generate(l, tn, None, true, None, n, c).unwrap();
        assert_eq!(
            adj("punainen", 38, Number::Plural, Case::Comitative),
            ["punaisine"]
        );
        assert_eq!(
            adj("nopea", 10, Number::Plural, Case::Comitative),
            ["nopeine"]
        );
        // Nouns keep the possessive citation, primary first.
        assert_eq!(
            generate(
                "talo",
                1,
                None,
                false,
                None,
                Number::Plural,
                Case::Comitative
            )
            .unwrap(),
            ["taloineen", "taloinensa"]
        );
    }

    // Ordinals (tn45): -nne-/-nte- oblique stems, -nsi- plural, -tta partitive. All forms
    // below were cross-checked against Voikko. Harmony follows the base (neljäs -> neljättä).
    #[test]
    fn ordinal_class_45() {
        let g = |l, n, c| one(l, 45, None, n, c);
        // kolmas: weak nne- (gen/ine), strong nte- (illat/essive), -tta partitive.
        assert_eq!(g("kolmas", Number::Singular, Case::Genitive), "kolmannen");
        assert_eq!(g("kolmas", Number::Singular, Case::Partitive), "kolmatta");
        assert_eq!(g("kolmas", Number::Singular, Case::Inessive), "kolmannessa");
        assert_eq!(g("kolmas", Number::Singular, Case::Illative), "kolmanteen");
        assert_eq!(g("kolmas", Number::Singular, Case::Essive), "kolmantena");
        assert_eq!(g("kolmas", Number::Plural, Case::Nominative), "kolmannet");
        assert_eq!(g("kolmas", Number::Plural, Case::Genitive), "kolmansien");
        assert_eq!(g("kolmas", Number::Plural, Case::Inessive), "kolmansissa");
        assert_eq!(g("kolmas", Number::Plural, Case::Partitive), "kolmansia");
        // kymmenes: longer base, same shape.
        assert_eq!(
            g("kymmenes", Number::Singular, Case::Genitive),
            "kymmenennen"
        );
        assert_eq!(
            g("kymmenes", Number::Singular, Case::Illative),
            "kymmenenteen"
        );
        assert_eq!(
            g("kymmenes", Number::Plural, Case::Inessive),
            "kymmenensissä"
        );
        // neljäs: front-harmony base -> -ttä, -nä, -ä.
        assert_eq!(g("neljäs", Number::Singular, Case::Partitive), "neljättä");
        assert_eq!(g("neljäs", Number::Singular, Case::Essive), "neljäntenä");
        assert_eq!(g("neljäs", Number::Plural, Case::Partitive), "neljänsiä");
        // mones (pronominal ordinal) declines through the same arm.
        assert_eq!(g("mones", Number::Singular, Case::Genitive), "monennen");
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
    fn agar_consonant_final_tn6() {
        // Consonant-final tn6 loanword (agar): same epenthetic -i- stem, bare nominative —
        // agari-: agarin / agaria / agarissa / agareissa (not the bare agarn / agarssa).
        let g = |n, c| one("agar", 6, None, n, c);
        assert_eq!(g(Number::Singular, Case::Nominative), "agar");
        assert_eq!(g(Number::Singular, Case::Genitive), "agarin");
        assert_eq!(g(Number::Singular, Case::Partitive), "agaria");
        assert_eq!(g(Number::Singular, Case::Inessive), "agarissa");
        assert_eq!(g(Number::Plural, Case::Inessive), "agareissa");
        // Regression: vowel-final tn6 unchanged.
        assert_eq!(
            one("paperi", 6, None, Number::Plural, Case::Inessive),
            "papereissa"
        );
    }

    #[test]
    fn nominals_decline_via_shared_classes() {
        // Adjectives and numerals are nominals using the same Kotus classes as nouns; the
        // generator is class-driven, so it declines them with no word-class awareness.
        assert_eq!(
            one("nopea", 15, None, Number::Singular, Case::Inessive),
            "nopeassa"
        ); // adjective, tn15
        assert_eq!(
            one("nopea", 15, None, Number::Plural, Case::Inessive),
            "nopeissa"
        );
        assert_eq!(
            one("kaunis", 41, None, Number::Singular, Case::Inessive),
            "kauniissa"
        ); // adjective, tn41
        assert_eq!(
            one("ensimmäinen", 38, None, Number::Singular, Case::Inessive),
            "ensimmäisessä"
        ); // numeral, tn38
        assert_eq!(
            one("biljoona", 10, None, Number::Singular, Case::Inessive),
            "biljoonassa"
        ); // numeral, tn10
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
        assert!(generate(
            "kevät",
            44,
            None,
            false,
            None,
            Number::Singular,
            Case::Genitive
        )
        .is_none());
    }
}
