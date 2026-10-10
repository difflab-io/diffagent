---
base_commit: '3b96f43613e797af487ce9360818962b7b0813f3'
---

# PoC: Remote orchestration

Experimental branch only. Do not merge into production or publish as a release.

**Status: incomplete, with no live end-to-end validation.** This branch contains an initial implementation, not a demonstrated working integration. No real provider login, GCP agent installation, model inference, remote commit, or remote push has been tested. The checks recorded below cover Python compilation, CLI smoke checks, mocked authorization/transport, and local Git fixtures with a fake agent.

## Purpose

Can a small CLI sign a user into GitHub, OpenAI, and OpenRouter, launch Codex or Pi on a dedicated GCP VPS, and commit changes on a new branch? This question remains unanswered. Live acceptance requires real sign-ins and one successful remote run per agent.

## Setup and tasks

Local: Python 3.11+ (tested 3.11.9), Git, gcloud, Codex, and optionally gh. No Python dependencies. Remote: dedicated trusted Linux VPS, Python 3.11+, Git, Node 22+ and npm. Bootstrap installs current Codex/Pi under ~/.local; versions are not pinned in this first experiment. Record its printed versions for live results.

| Task | Command | Observed result |
| --- | --- | --- |
| Build | `mise run build` | Python compilation passed |
| Test | `mise run test` | Seven fixture/mocked tests passed; no live integration tested |
| CLI | `mise run run:cli` | Help command passed |
| Status | `mise run run:status` | Passed; all three providers not connected |

Without mise use `python3 -m orchestration`. Credentials are private plaintext files under ~/.local/state/orchestration (0700 directory, 0600 files), overridable with ORCH_STATE_DIR. Status reports presence, not verified entitlement. This is not an enterprise credential vault.

### Sign in and OAuth apps

Register a GitHub OAuth App and enable device flow. Its client ID needs no client secret for device login. Scopes are repo/read:user. Expiring tokens require signing in again; automatic refresh is not implemented. Omit --client-id to use gh's own OAuth application instead.

```sh
python3 -m orchestration login github --client-id YOUR_GITHUB_CLIENT_ID
python3 -m orchestration login openai
python3 -m orchestration login openrouter
python3 -m orchestration status
```

OpenAI uses Codex's official device login, with an isolated PoC CODEX_HOME. Enable device authentication in ChatGPT settings when needed. App-owned OpenAI OAuth registration remains a follow-up, distinct from this official CLI login. OpenRouter uses public S256 PKCE and a localhost callback at a random port/path. It returns a user-controlled API key and spends API credits; it needs no custom client ID in this flow.

Sources checked October 9, 2026: [GitHub device flow](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps), [Codex login](https://developers.openai.com/codex/auth), [OpenRouter PKCE](https://openrouter.ai/docs/guides/overview/auth/oauth), [app-owned OpenAI delegation](https://developers.openai.com/siwc/token-sharing-open-source).

### Remote host

Select a billing-enabled GCP project, intended account, and dedicated Linux instance. The CLI does not provision/delete cloud resources. An example creation command, after choosing the project and zone:

```sh
gcloud compute instances create orchestration-demo --project YOUR_PROJECT --zone YOUR_ZONE --machine-type e2-standard-2 --image-family ubuntu-2404-lts-amd64 --image-project ubuntu-os-cloud
```

Install Python, Git, and Node 22+ on the host, then install agents:

```sh
python3 -m orchestration bootstrap --project YOUR_PROJECT --instance orchestration-demo --zone YOUR_ZONE
```

Use --iap for IAP access if configured; otherwise normal gcloud SSH access applies. The user signed in with gcloud. The last check found that the configured project returned "not found" when listing instances. No target instance or zone has been selected or tested.

### Launch and commit

Use a repo authorized for the experiment, a fresh poc/... branch, and a model available to your account. Codex uses OpenAI login; Pi uses OpenRouter. These are two distinct combinations, not a complete agent/provider matrix.

```sh
python3 -m orchestration run --project YOUR_PROJECT --instance orchestration-demo --zone YOUR_ZONE --repo YOUR_OWNER/YOUR_REPO --branch poc/codex-demo --agent codex --model YOUR_CODEX_MODEL --prompt 'Add a greeting script and verify it works.' --author-name 'Your Name' --author-email 'YOUR_EMAIL'
python3 -m orchestration run --project YOUR_PROJECT --instance orchestration-demo --zone YOUR_ZONE --repo YOUR_OWNER/YOUR_REPO --branch poc/pi-demo --agent pi --model YOUR_OPENROUTER_MODEL --prompt 'Add a greeting script and verify it works.' --author-name 'Your Name' --author-email 'YOUR_EMAIL'
```

The workflow clones the requested base (main by default), starts the agent, and stages/commits only after success. --push explicitly publishes the commit; otherwise it stays on the VPS. Workspaces remain under ~/orchestration-runs/run-*/repo, including failures. Commit hooks are disabled for the workflow-owned commit.

Credentials travel through SSH stdin, not command arguments or Git remote URLs. Temporary auth directories are cleaned on normal completion/failure; abrupt termination can leave them. A trusted dedicated VPS is required: Pi executes with the remote user's permissions; Codex uses workspace-write. Agent-executed code can access process credentials. Asking the agent not to commit/push is not an enforced boundary. Agent output is not secret-redacted. This is a single-user demonstration, not tenant isolation.

## Code map

- orchestration/__main__.py: argparse commands and credential selection.
- orchestration/auth.py: provider sign-in and local storage.
- orchestration/remote.py: SSH transport and agent installation.
- orchestration/worker.py: standalone clone/agent/commit lifecycle.
- tests/test_workflow.py: real Git fixture, fake-agent success/failure, credential permissions.

## Alternatives and experiments

The implementation currently has direct provider connections and a single GCP SSH runner. Kinde login and identity linking, GitHub App installations, LiteLLM routing, Claude Code subscription passthrough, managed-agent adapters, and enterprise/SaaS deployment were discussed but are not implemented.

| Experiment | Result |
| --- | --- |
| Standalone runner clones, branches, commits using a fake agent | Passed with actual local Git repository |
| Agent failure | Preserved checkout; no workflow commit |
| Real consent flows | Not run |
| GCP SSH/bootstrap | Not run; host selection pending |
| Codex/Pi inference | Not run; authentication/model selection pending |

## Learnings

Local fixtures show that the standalone worker can produce a Git commit after a fake agent exits successfully and leave changes uncommitted after failure. They do not establish that real agents or remote transport work. Live model access, consent flows, OpenRouter callback acceptance, Codex remote sandbox compatibility, and SSH stdin transport remain unverified. No conclusion about the proposed architecture's viability has been established.

## Compare with main at branch creation

This branch was renamed from the clean slate at origin/main. Keep base_commit immutable. Compare committed work with `git diff 3b96f43613e797af487ce9360818962b7b0813f3 HEAD`. Before committing, include untracked prototype files in the review as well.

## Disposition

Preserve this incomplete implementation for further work. Complete live acceptance with the user's selected accounts/VPS/repo and record actual results before treating the PoC as demonstrated or ready to freeze. Do not merge this experiment.
