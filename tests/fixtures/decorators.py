"""Test fixture for decorator ignore patterns."""


class router:  # noqa: N801 — lowercase on purpose: mimics FastAPI's `router` instance for decorator-pattern tests
    @staticmethod
    def get(path):
        def decorator(func):
            return func

        return decorator

    @staticmethod
    def post(path):
        def decorator(func):
            return func

        return decorator


@router.get("/items")
def list_items():
    return []


@router.post("/items")
def create_item():
    return {}


def undecorated_unused():
    return "dead"
