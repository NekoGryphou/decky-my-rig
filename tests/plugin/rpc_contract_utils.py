"""Reads the RPC contract from both sides: the TypeScript names and signatures
and the Python `Plugin` methods."""

import ast
import re

# - `watchSnapshot: "watch_snapshot",` in RPC_METHODS.
_METHOD = re.compile(r'(\w+):\s*"(\w+)"')
# - `deletePc: { args: [identifier: string]; result: DeleteResult };`
_SIGNATURE = re.compile(r"(\w+):\s*\{\s*args:\s*\[(.*?)\];\s*result:", re.DOTALL)
_ARGUMENT = re.compile(r"(\w+)\??:")
_CAMEL = re.compile(r"(?<!^)(?=[A-Z])")


def snake_case(name: str) -> str:
    return _CAMEL.sub("_", name).lower()


def typescript_contract(consts: str, types: str) -> dict[str, list[str]]:
    """Wire method name -> argument names in order, from api.consts.ts and
    the RpcSignatures interface of api.types.ts."""
    wire = dict(_METHOD.findall(consts))
    arguments = {
        key: [snake_case(name) for name in _ARGUMENT.findall(labels)]
        for key, labels in _SIGNATURE.findall(types)
    }
    return {method: arguments.get(key, []) for key, method in wire.items()}


def python_contract(source: str) -> dict[str, list[str]]:
    """Public method name -> argument names in order, from main.py's Plugin."""
    plugin = next(
        node
        for node in ast.parse(source).body
        if isinstance(node, ast.ClassDef) and node.name == "Plugin"
    )
    return {
        node.name: [argument.arg for argument in node.args.args[1:]]
        for node in plugin.body
        if isinstance(node, ast.AsyncFunctionDef) and not node.name.startswith("_")
    }


def drift(typescript: dict[str, list[str]], python: dict[str, list[str]]) -> list[str]:
    """Every difference between the two sides, one line each."""
    problems = [
        f"{method} is not defined by the Plugin"
        for method in typescript
        if method not in python
    ]
    problems += [
        f"{method} is not declared in TypeScript"
        for method in python
        if method not in typescript
    ]
    problems += [
        f"{method} takes {python[method]} but TypeScript sends {typescript[method]}"
        for method in typescript
        if method in python and python[method] != typescript[method]
    ]
    return problems
