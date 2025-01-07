# actions-test

Test for github-forgejo action bridge

The idea is to use the [repository_dispatch](https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#repository_dispatch)
workflow trigger to trigger workflows from [Forgejo](https://forge.aftix.xyz/aftix/actions-test) webhooks.

This uses a very simple HTTP proxy running locally to the forgejo instance which wraps
the forgejo webhook into the expected JSON format for `repository_dispatch`.

The HTTP proxy in this repo expects to be run with
environment variable LISTEN_PORT set to the desired port.
It supports TLS by setting both KEY_PATH and CERT_PATH in
the environment.
