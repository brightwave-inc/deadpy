"""File with definitions that should be saved by whitelist."""


def whitelisted_function():
    """Saved by whitelist."""
    return "alive"


def not_whitelisted():
    """Not in whitelist — should be flagged."""
    return "dead"


class WhitelistedClass:
    def whitelisted_method(self):
        """Saved by whitelist."""
        pass
