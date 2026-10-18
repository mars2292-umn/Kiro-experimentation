"""``python -m jsinfer``: the ``jsinfer`` command-line interface (``jsinfer.cli.main``), exiting with its status."""

import sys

from jsinfer.cli import main

if __name__ == "__main__":  # importing the module runs nothing
    sys.exit(main())
