#!/usr/bin/env python3
"""Export the irregulars catalog as content for keinonto-web (pSEO + social).

Turns three QA artifacts into one structured JSON the web side templates into
one-page-per-word ("miksi 'ko'oissa' kirjoitetaan heittomerkillä?") and social snippets:

  - crates/keinontolibrary-rules/exceptions.toml — the registry, each row carrying a
    human-written `reason` (aika→ajan k:j, veli→velj-, …).
  - data/*-overrides.jsonl — the probe-minted sidecars (harmony, comitative, citation).
  - qa/accepted.jsonl — words where Kotus and Voikko disagree, each with a reason.

Forms come ONLY from the `rules` leg of qa/generated.jsonl (rule generator + registry —
never the corpus lookup), so the committed file carries no corpus-derived data; it is
licensing-clean (linguistic facts of a few hundred well-known irregulars).

Output: data/content/irregulars.json. Run after a full QA loop (needs qa/generated.jsonl).
"""
import argparse
import json
import tomllib
from collections import defaultdict

CASE_ORDER = ['nominative', 'genitive', 'partitive', 'accusative', 'inessive', 'elative',
              'illative', 'adessive', 'ablative', 'allative', 'essive', 'translative',
              'abessive', 'comitative', 'instructive']

# Family tags, matched against the lemma's reason text (first match wins). Each is a
# hub-page topic on keinonto-web.
FAMILIES = [
    ('kj-gradation', 'k:j-astevaihtelu', 'k→j', 'k:j gradation'),
    ('velj-stem', 'veli-tyyppi', 'velj-', 'lj- oblique'),
    ('k-insertion', 'k:n lisäys', 'k-insertion', 'k-insertion'),
    ('pronoun', 'pronominit', 'pronoun', 'pronoun'),
    ('possessive-pronominal', 'omistusliite-pronominit', '-laiseni', 'possessive-marked'),
    ('plurale-tantum', 'monikkosanat', 'plurale tantum', 'tantum'),
    ('compound-head', 'yhdyssanan pää', 'compound head', 'compound'),
    ('loanword-harmony', 'lainasanojen vokaalisointu', 'harmony', 'harmony'),
    ('foreign-citation', 'vieraskieliset sitaatit', "parfait'n", 'citation'),
    ('comitative-style', 'komitatiivi', '-ine', 'comitative'),
    ('singleton', 'omat luokkansa', 'singleton', 'singleton'),
    ('numeral', 'lukusanat', 'numeral', 'numeral'),
    ('accented-loan', 'aksenttilainat', 'accent', 'accented'),
]


def family_of(reason, lemma, in_harmony, in_citation, in_comitative):
    r = (reason or '').lower()
    if in_citation:
        return 'foreign-citation'
    if in_harmony:
        return 'loanword-harmony'
    if in_comitative:
        return 'comitative-style'
    for fam_id, _fi, _ex, needle in FAMILIES:
        if needle.lower() in r:
            return fam_id
    return 'other'


FAMILY_META = {fid: {'id': fid, 'title': fi, 'example': ex}
               for fid, fi, ex, _ in FAMILIES}
FAMILY_META['other'] = {'id': 'other', 'title': 'muut', 'example': ''}

# Cross-cutting "war stories" — the rules behind the surprising spellings. Hand-authored;
# the strongest social material.
STORIES = [
    {'id': 'taika-not-compound', 'title': "taika ei ole t + aika",
     'body': "taika taipuu taian (säännöllinen k-kato), ei *tajan — vaikka se näyttää "
             "yhdyssanalta -aika. Yhdyssanan tunnistus vaatii vähintään kaksikirjaimisen "
             "etuosan (yöaika → yöajan).",
     'examples': ['taika → taian', 'yöaika → yöajan', 'adventtiaika → adventtiajan']},
    {'id': 'apostrophe-orthography', 'title': "vaa'an mutta vaaoissa",
     'body': "k-kato samojen vokaalien välissä kirjoitetaan heittomerkillä (vaaka → "
             "vaa'an), mutta monikossa vokaali pyöristyy eikä heittomerkkiä tarvita "
             "(vaaoissa). Konsonantin jäljessä syntyy pitkä vokaali (koko → koon), joka "
             "monikossa avautuu uudelleen: ko'oissa.",
     'examples': ["vaaka → vaa'an, vaaoissa", 'koko → koon, ko\'oissa', 'rako → raon']},
    {'id': 'last-strong-vowel', 'title': "afääriä mutta countrya",
     'body': "Vokaalisoinnun ratkaisee viimeinen vahva vokaali (a/o/u vs ä/ö): afääri → "
             "afääriä. y ei ratkaise: englannin lainoissa se ei ole etuvokaali "
             "(country → countrya, jury → jurya).",
     'examples': ['afääri → afääriä', 'tyranni → tyrannia', 'country → countrya']},
    {'id': 'compound-harmony', 'title': "antigeenissä, ei antigeenissa",
     'body': "Yhdyssanan sointu seuraa loppuosaa: anti+geeni → antigeenissä, vaikka "
             "kirjoitusasussa on takavokaali a. Yksinkertaisissa lainoissa (harakiri, "
             "alumiini) sointu pysyy takaisena.",
     'examples': ['antigeeni → antigeenissä', 'ajanviete → ajanvietettä']},
    {'id': 'pronoun-clitics', 'title': "jossakin — liite taipuu sisällä",
     'body': "Osa pronomineista taivuttaa liitteen sisällä: jokin → jossakin, kukaan → "
             "kenenkään. Toisilla liite pysyy lopussa: kumpikin → kummankin.",
     'examples': ['jokin → jossakin', 'kukaan → kenenkään', 'kumpikin → kummankin',
                  'kuka → kenet']},
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--dump', default='qa/generated.jsonl')
    ap.add_argument('--registry', default='crates/keinontolibrary-rules/exceptions.toml')
    ap.add_argument('--accepted', default='qa/accepted.jsonl')
    ap.add_argument('--out', default='data/content/irregulars.json')
    args = ap.parse_args()

    # Reasons + lemma set from the registry.
    reg = tomllib.load(open(args.registry, 'rb'))
    reasons, tn_of = {}, {}
    for e in reg['exception']:
        lemma = e['lemma']
        # Prefer a substantive reason over "defective slot" notes.
        if lemma not in reasons or 'defective' in reasons[lemma]:
            reasons.setdefault(lemma, e.get('reason', ''))
            if 'defective' not in e.get('reason', ''):
                reasons[lemma] = e.get('reason', '')
        tn_of[lemma] = e.get('tn')
    reg_lemmas = set(reasons)  # snapshot before tantum reasons are merged in below

    def load_sidecar(path):
        out = {}
        try:
            for line in open(path, encoding='utf-8'):
                if line.strip():
                    out[json.loads(line)['lemma']] = json.loads(line)
        except FileNotFoundError:
            pass
        return out

    harmony = load_sidecar('data/harmony-overrides.jsonl')
    comitative = load_sidecar('data/comitative-overrides.jsonl')
    citation = load_sidecar('data/citation-overrides.jsonl')

    accepted = {}
    for line in open(args.accepted, encoding='utf-8'):
        if not line.strip():
            continue
        row = json.loads(line)
        lemma = row['match'].split('|')[0]
        if lemma != '*':
            accepted.setdefault(lemma, row['reason'])

    # The content universe: genuinely irregular words worth a page — the registry, the
    # harmony and citation sidecars, and the Kotus/Voikko-disagreement accepted set. The
    # comitative sidecar is excluded as a universe source (it is a routine style flag on
    # ~3000 ordinary nouns); it still informs family tags below. Forms from the rules
    # leg only (no corpus).
    base_special = set(reasons) | set(harmony) | set(citation) | set(accepted)
    # Plurale tantums (sakset, talkoot, rattaat): no singular but a generated plural —
    # iconic "words with no singular" content, synthesized reason. Detected from the dump.
    sg_nom_missing, pl_present = set(), set()
    all_forms = defaultdict(dict)
    lemma_tn = {}
    for line in open(args.dump, encoding='utf-8'):
        row = json.loads(line)
        lemma, rules = row['lemma'], row.get('rules')
        has = bool(rules and rules.get('status') != 'missing' and rules.get('variants'))
        if row['number'] == 'singular' and row['case'] == 'nominative' and not has:
            sg_nom_missing.add(lemma)
        if row['number'] == 'plural' and row['case'] == 'nominative' and has:
            pl_present.add(lemma)
        if has:
            all_forms[lemma][(row['number'], row['case'])] = rules['variants']
            lemma_tn[lemma] = row['tn']
    tantums = sg_nom_missing & pl_present
    for lemma in tantums:
        reasons.setdefault(lemma, 'plurale tantum: a plural-only word with no singular (like sakset)')

    special = base_special | tantums
    forms = {lemma: slots for lemma, slots in all_forms.items() if lemma in special}

    words = []
    fam_counts = defaultdict(int)
    for lemma in sorted(special):
        slots = forms.get(lemma)
        if not slots:
            continue  # no rule-generated forms (pure corpus lemma) — skip, keep it clean
        why = reasons.get(lemma) or accepted.get(lemma) or ''
        if not why:
            if lemma in citation:
                why = "declines on its pronunciation behind a separator (parfait'n, cd:n)"
            elif lemma in harmony:
                why = "compound vowel harmony follows the final component"
            elif lemma in comitative:
                why = "modifier comitative is the bare -ine, not -ineen"
        fam = family_of(why, lemma, lemma in harmony, lemma in citation,
                        lemma in comitative)
        fam_counts[fam] += 1
        table = [{'number': n, 'case': c, 'forms': slots[(n, c)]}
                 for n in ('singular', 'plural') for c in CASE_ORDER
                 if (n, c) in slots]
        nom = slots.get(('singular', 'nominative'), [lemma])[0]
        # Headline: the slot that best *shows* this family's quirk — harmony shows in a
        # vowel-ending case, the comitative/tantum families in their plural slots, the
        # rest in the singular genitive — falling back to the first surprising form.
        preferred = {
            'loanword-harmony': [('singular', 'inessive'), ('singular', 'partitive')],
            'comitative-style': [('plural', 'comitative')],
            'possessive-pronominal': [('plural', 'comitative'), ('singular', 'partitive')],
            'plurale-tantum': [('plural', 'inessive'), ('plural', 'genitive')],
            'numeral': [('singular', 'genitive')],
        }.get(fam, [('singular', 'genitive')])
        headline = None
        for key in preferred:
            if key in slots and slots[key][0] != nom:
                headline = {'number': key[0], 'case': key[1], 'form': slots[key][0]}
                break
        if headline is None:
            for entry in table:
                f = entry['forms'][0]
                if not f.startswith(nom[:max(2, len(nom) - 2)]):
                    headline = {'number': entry['number'], 'case': entry['case'], 'form': f}
                    break
        words.append({
            'lemma': lemma,
            'tn': lemma_tn.get(lemma, tn_of.get(lemma)),
            'family': fam,
            'why': why,
            'source': ('registry' if lemma in reg_lemmas else
                       'harmony' if lemma in harmony else
                       'citation' if lemma in citation else
                       'tantum' if lemma in tantums else
                       'comitative' if lemma in comitative else 'accepted'),
            'nominative': nom,
            'headline': headline,
            'forms': table,
        })

    families = [{**FAMILY_META[f], 'count': fam_counts[f]}
                for f in sorted(fam_counts, key=lambda f: -fam_counts[f])]
    out = {
        'schema': 1,
        'note': 'Forms are rule/registry-generated (no corpus data). See LICENSING.md.',
        'families': families,
        'stories': STORIES,
        'words': words,
    }
    import os
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, 'w', encoding='utf-8') as fh:
        json.dump(out, fh, ensure_ascii=False, indent=1)
    print(f"{len(words)} words, {len(families)} families, {len(STORIES)} stories -> {args.out}")


if __name__ == '__main__':
    main()
