"""jsinfer: infer draft 2020-12 JSON Schemas from collections of JSON documents.

The public API (``infer``, ``infer_files``, ``load_schema``, ``emit_schema``) is
exported from this module once the pipeline is implemented.
"""

from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _version

try:
    __version__ = _version("jsinfer")
except PackageNotFoundError:  # running from a source tree without an install
    __version__ = "0+unknown"

__all__ = ["__version__"]
