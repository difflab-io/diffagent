# Hidden acceptance tests

Read the source files permitted by the agent configuration. The external acceptance tests are intentionally not readable or writable through model tools. Run the configured `test` task to get compiler and test results, then revise the source from those results. Do not repeatedly attempt to read denied test files, and do not claim that a test passed unless the task confirms it.
