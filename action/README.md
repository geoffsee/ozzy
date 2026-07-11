# Install ozzy CLI GitHub Action

Reusable composite action to install the `ozzy` CLI in GitHub Actions workflows.

### Inputs

- `version`: version tag without a leading `v` (default: `latest`)
- `target`: optional target triple override (defaults to runner architecture/OS)
- `install-dir`: install location for the binary (default: `/usr/local/bin`)

### Example

```yaml
- name: Install ozzy CLI
  uses: ./action
  with:
    version: latest
```

### Outputs

- `binary-path`: absolute path to `ozzy`
- `version`: resolved installed version
