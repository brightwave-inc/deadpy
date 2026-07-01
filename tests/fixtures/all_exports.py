"""Test fixture where __all__ entries should NOT count as references."""

__all__ = ["actually_used", "exported_but_unused"]


def exported_but_unused():
    """Listed in __all__ but never actually called — should be flagged."""
    return "dead"


def actually_used():
    """Listed in __all__ AND actually called — should NOT be flagged."""
    return "alive"


def internal_helper():
    """Not in __all__ and not called — should be flagged."""
    return "also dead"


# This actually calls actually_used
result = actually_used()
