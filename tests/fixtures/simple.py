"""Simple test fixture with dead and alive code."""

from os.path import join


def used_function():
    """This function is called elsewhere."""
    return join("a", "b")


def unused_function():
    """This function is never called."""
    return 42


class UsedClass:
    def method_a(self):
        pass


class UnusedClass:
    def method_b(self):
        pass


# This calls used_function and references UsedClass
result = used_function()
instance = UsedClass()
