"""Standalone remote runner. Receives one JSON payload on stdin."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def agent_command(agent, model, prompt):
    instruction = prompt + '\nDo not commit or push. The workflow will commit your changes.'
    if agent == 'codex':
        return ['codex', 'exec', '--ignore-user-config', '--ephemeral', '--json',
                '-s', 'workspace-write', '-m', model, '--', instruction]
    return ['pi', '--print', '--mode', 'json', '--no-session', '--no-extensions',
            '--no-skills', '--no-prompt-templates', '--no-themes',
            '--provider', 'openrouter', '--model', model, '--', instruction]


def validate(payload):
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', payload['repo']):
        raise ValueError('Repo must be owner/name')
    if payload['agent'] not in ('codex', 'pi'):
        raise ValueError('Unknown agent')
    branch = payload['branch']
    if not branch.startswith('poc/') or '..' in branch or '@{' in branch:
        raise ValueError('Demo branches must start with poc/ and be valid Git refs')
    subprocess.run(['git', 'check-ref-format', '--branch', branch], check=True, stdout=subprocess.DEVNULL)


def run(payload):
    validate(payload)
    root = Path.home() / 'orchestration-runs'
    root.mkdir(mode=0o700, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix='run-', dir=root))
    repo = run_dir / 'repo'
    with tempfile.TemporaryDirectory(prefix='orchestration-auth-') as tmp:
        auth = Path(tmp)
        askpass = auth / 'askpass'
        askpass.write_text('#!/bin/sh\ncase "$1" in *Username*) echo x-access-token;; *) printf "%s\\n" "$ORCH_GITHUB_TOKEN";; esac\n')
        askpass.chmod(0o700)
        codex = auth / 'codex'
        codex.mkdir(mode=0o700)
        if payload.get('codex_auth'):
            (codex / 'auth.json').write_text(json.dumps(payload['codex_auth']))
            (codex / 'auth.json').chmod(0o600)
        pi = auth / 'pi'
        pi.mkdir(mode=0o700)
        env = dict(os.environ, PATH=str(Path.home() / '.local/node_modules/.bin') + ':' + os.environ['PATH'],
                   GIT_ASKPASS=str(askpass), GIT_TERMINAL_PROMPT='0',
                   ORCH_GITHUB_TOKEN=payload['github_token'], CODEX_HOME=str(codex),
                   PI_CODING_AGENT_DIR=str(pi), OPENROUTER_API_KEY=payload.get('openrouter_key', ''))

        def git(*args, capture=False):
            result = subprocess.run(['git', '-c', 'credential.helper=', *args], cwd=repo if repo.exists() else run_dir,
                                    env=env, check=True, text=True, stdout=subprocess.PIPE if capture else None)
            return result.stdout.strip() if capture else None

        git('clone', '--branch', payload['base'], '--single-branch',
            'https://github.com/' + payload['repo'] + '.git', str(repo))
        git('switch', '-c', payload['branch'])
        git('config', 'user.name', payload['author_name'])
        git('config', 'user.email', payload['author_email'])
        print('Workspace:', repo, flush=True)
        subprocess.run(agent_command(payload['agent'], payload['model'], payload['prompt']), cwd=repo, env=env,
                       check=True, timeout=payload['timeout'])
        if git('branch', '--show-current', capture=True) != payload['branch']:
            raise RuntimeError('Agent changed branch; refusing workflow commit')
        git('add', '--all')
        changes = git('diff', '--cached', '--name-only', capture=True)
        if not changes:
            print('No changes to commit', flush=True)
            return
        git('-c', 'core.hooksPath=/dev/null', 'commit', '-m', payload['message'])
        print('Commit:', git('rev-parse', 'HEAD', capture=True), flush=True)
        if payload['push']:
            git('push', 'origin', 'HEAD:refs/heads/' + payload['branch'])
        print('Workspace preserved:', repo, flush=True)


if __name__ == '__main__':
    run(json.load(sys.stdin))
