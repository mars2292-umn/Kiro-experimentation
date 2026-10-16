"""Shared pytest configuration.

Hypothesis profiles:

- ``ci`` (default): 100 examples per property, derandomized so CI runs are reproducible.
- ``nightly``: 500 examples per property, randomized to explore new inputs each run.

Select a profile with the ``HYPOTHESIS_PROFILE`` environment variable or with
Hypothesis's ``--hypothesis-profile`` pytest option (the option wins).
"""

from __future__ import annotations

import os

from hypothesis import settings

# Pipeline-level properties run whole inferences per example, so wall-clock
# deadlines would only produce timing flakes.
settings.register_profile("ci", max_examples=100, derandomize=True, deadline=None, print_blob=True)
settings.register_profile("nightly", max_examples=500, deadline=None, print_blob=True)
settings.load_profile(os.environ.get("HYPOTHESIS_PROFILE", "ci"))
