"""Transport a runner and its input over gcloud SSH. Secrets only use stdin."""
import json
from pathlib import Path
import shlex
import subprocess


def ssh_args(args):
    cmd = ['gcloud', 'compute', 'ssh', args.instance, '--project', args.project, '--zone', args.zone]
    if args.iap:
        cmd.append('--tunnel-through-iap')
    return cmd


def launch(args, payload):
    source = Path(__file__).with_name('worker.py').read_text()
    command = 'python3 -c ' + shlex.quote(source)
    subprocess.run(ssh_args(args) + ['--command', command, '--', '-T'],
                   input=json.dumps(payload), text=True, check=True)


def bootstrap(args):
    command = ('set -eu; command -v python3; command -v git; command -v node; '
               'mkdir -p "$HOME/.local"; npm install --prefix "$HOME/.local" '
               '@openai/codex @earendil-works/pi-coding-agent; '
               '"$HOME/.local/node_modules/.bin/codex" --version; '
               '"$HOME/.local/node_modules/.bin/pi" --version')
    subprocess.run(ssh_args(args) + ['--command', command, '--', '-T'], check=True)
