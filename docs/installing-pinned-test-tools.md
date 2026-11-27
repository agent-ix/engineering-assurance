# Installing pinned test tools

The repository-owned registry at `spec/test-tool-downloads.json` records the
approved version, source, artifact digest, license, rights owner, purpose, and
consumer for each external test tool. The content-rights check admits those
source URLs only through the registry and its listed consumers.

On Linux x86-64, install the reviewed Syft release with:

```sh
scripts/install-test-syft.sh
```

The installer reads the source URL and SHA-256 from the registry, verifies the
download before extraction, and installs only the `syft` executable to
`$HOME/.local/bin/syft`. Pass an absolute directory as the first argument to
choose another destination. The Syft conformance tests then use that binary
through `EA_TEST_SYFT`; they do not download tools during a test run.
