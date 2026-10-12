# Manual tests

Experiments with Yalom message generation; these files are not used by the Rust backend.

Run from the repository root:

```bash
./agent/manual_test/test.sh --intent observation --time-focus --count 5
```

`test.sh` reads the production prompt files from `../prompts/`.
`request.json` is the OpenAI API request template.
`corpus.json` is only sample data for the `--curated` flag; the backend reads curated messages from MySQL.
