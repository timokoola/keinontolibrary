"""keinontolibrary — decline Finnish nouns.

Data-backed (a Voikko-verified reference corpus) with a rule-based fallback over the Kotus
declension classes. The compiled engine and its data ship inside this wheel, so:

    >>> import keinontolibrary
    >>> keinontolibrary.decline("hevonen", "plural", "inessive")
    ['hevosissa']
    >>> keinontolibrary.paradigm("talo")["singular"]["inessive"]
    ['talossa']
"""

from __future__ import annotations

from importlib import resources
from typing import Dict, List

from ._keinontolibrary import Inflector, __version__

__all__ = ["decline", "paradigm", "Inflector", "__version__"]

_ARTIFACT = "keinontolibrary.bin"
_OVERLAY = "overlay.jsonl"

_engine: "Inflector | None" = None


def _default() -> Inflector:
    """Lazily open the engine backed by the data bundled in this package."""
    global _engine
    if _engine is None:
        files = resources.files(__package__)
        # as_file materializes the resources to real paths (needed when installed from a zip).
        with resources.as_file(files / _ARTIFACT) as artifact, \
                resources.as_file(files / _OVERLAY) as overlay:
            _engine = Inflector(str(artifact), str(overlay))
    return _engine


def decline(word: str, number: str, case: str) -> List[str]:
    """Decline ``word`` into one ``(number, case)`` slot.

    ``number`` is ``"singular"``/``"plural"``; ``case`` is an English case name
    (``"nominative"``, ``"genitive"``, ``"inessive"``, ...). Returns the surface form(s);
    raises ``KeyError`` for an unknown word, ``ValueError`` for bad arguments/ambiguity.
    """
    return _default().decline(word, number, case)


def paradigm(word: str) -> Dict[str, Dict[str, List[str]]]:
    """Return the full paradigm for ``word`` as ``{number: {case: [forms...]}}``."""
    return _default().paradigm(word)
