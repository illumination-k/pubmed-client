Github action CI is failing.

Please fix the issues so that all checks pass successfully.

You first search which runs are failing by listing all runs on the current branch:

```bash
gh run list --branch $(git branch --show-current) -R $(gh repo view --json nameWithOwner --jq '.nameWithOwner')
```

Then you inspect the logs of the failed runs to identify the problems:

```bash
gh run view <run-id> --log --failed
```

And resolve the issues.
