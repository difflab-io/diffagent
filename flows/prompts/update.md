Request: {{ params.request }}
Plan: {{ steps.plan.output }}
Last test output: {{ steps.evaluate.output }}

Inspect the failed test output and source files that the configured tools permit you to read. Hidden acceptance tests are not readable. Fix the cause, then let the flow rerun tests. If you cannot fix it safely, explain the blocker rather than claiming success.
