"""Tests for infra/gcp.tf startup_probe configuration (search-front task 016).

From search-front/scripts/infra_probe_test.py, unchanged but for this
paragraph: infra/gcp.tf keeps search-front's TCP startup probe.

The startup probe must satisfy:
1. timeout_seconds < period_seconds (gcloud requirement)
2. period_seconds × failure_threshold ≥ 60 (margin for slow cold pull)

Run: python3 scripts/infra_probe_test.py
"""

import pathlib
import re
import unittest


def parse_startup_probe(gcp_tf_content):
    """Parse the startup_probe block from gcp.tf.
    
    Returns a dict with timeout_seconds, period_seconds, failure_threshold.
    Raises ValueError if the block is not found or malformed.
    """
    # Find the startup_probe block
    lines = gcp_tf_content.split('\n')
    start_idx = None
    for i, line in enumerate(lines):
        if 'startup_probe {' in line:
            start_idx = i
            break
    
    if start_idx is None:
        raise ValueError("startup_probe block not found in gcp.tf")
    
    # Find the end of the block (closing brace)
    brace_count = 1
    end_idx = start_idx
    
    for i in range(start_idx + 1, len(lines)):
        line = lines[i]
        brace_count += line.count('{') - line.count('}')
        
        if brace_count == 0:
            end_idx = i
            break
    
    probe_block = '\n'.join(lines[start_idx:end_idx + 1])
    
    # Parse each field
    values = {}
    for field in ('timeout_seconds', 'period_seconds', 'failure_threshold'):
        field_match = re.search(
            rf'{field}\s*=\s*(\d+)',
            probe_block
        )
        if field_match:
            values[field] = int(field_match.group(1))
        else:
            raise ValueError(f"{field} not found in startup_probe block")
    
    return values


class StartupProbeConfiguration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        gcp_tf_path = pathlib.Path(__file__).resolve().parent.parent / "infra" / "gcp.tf"
        cls.probe_values = parse_startup_probe(gcp_tf_path.read_text())
    
    def test_timeout_is_less_than_period(self):
        """gcloud requires timeout_seconds < period_seconds."""
        self.assertLess(
            self.probe_values['timeout_seconds'],
            self.probe_values['period_seconds'],
            f"timeout_seconds ({self.probe_values['timeout_seconds']}) must be less than "
            f"period_seconds ({self.probe_values['period_seconds']})"
        )
    
    def test_period_times_threshold_covers_60_second_margin(self):
        """period_seconds × failure_threshold must be ≥ 60 for slow cold pull."""
        margin = self.probe_values['period_seconds'] * self.probe_values['failure_threshold']
        self.assertGreaterEqual(
            margin,
            60,
            f"period_seconds ({self.probe_values['period_seconds']}) × "
            f"failure_threshold ({self.probe_values['failure_threshold']}) = {margin} "
            f"must be ≥ 60"
        )


if __name__ == "__main__":
    unittest.main()
