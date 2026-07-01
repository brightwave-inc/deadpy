"""Test fixture for dunder method auto-whitelisting."""


class MyClass:
    def __init__(self):
        self.value = 0

    def __str__(self):
        return str(self.value)

    def __repr__(self):
        return f"MyClass({self.value})"

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def unused_method(self):
        """Regular method that is never called — should be flagged."""
        return "dead"
