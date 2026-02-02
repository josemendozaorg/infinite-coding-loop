# Engineer Task: Verify Code
Source Files: {{source_content}}

Please verify that the implemented code works as expected.
1. Compile the code using `rustc` or check it with `cargo`.
2. Run the resulting executable and check the output.

{{schema}}

Provide a JSON response with:
- "score": 1.0 if it works perfectly, < 1.0 otherwise.
- "feedback": "Detailed explanation of the test results."
- "output": "The actual output from the program."
