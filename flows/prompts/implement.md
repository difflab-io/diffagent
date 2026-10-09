Request: {{ params.request }}
Plan: {{ steps.plan.output }}

Implement the request using the configured file tools. Read source files first. Do not pretend that a write succeeded before the tool confirms it. The next step will run the configured test task.
