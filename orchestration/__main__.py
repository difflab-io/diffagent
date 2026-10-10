import argparse
import json
import subprocess
import time
import urllib.error

from . import auth, remote


def parser():
    root = argparse.ArgumentParser(description='Sign in and run coding agents on a dedicated GCP VPS')
    commands = root.add_subparsers(dest='command', required=True)
    login = commands.add_parser('login')
    login.add_argument('provider', choices=['github', 'openai', 'openrouter'])
    login.add_argument('--client-id', help='Your GitHub OAuth App ID, with device flow enabled')
    commands.add_parser('status')
    for name in ['bootstrap', 'run']:
        cmd = commands.add_parser(name)
        cmd.add_argument('--project', required=True)
        cmd.add_argument('--instance', required=True)
        cmd.add_argument('--zone', required=True)
        cmd.add_argument('--iap', action='store_true')
        if name == 'run':
            cmd.add_argument('--repo', required=True, help='GitHub owner/name')
            cmd.add_argument('--base', default='main')
            cmd.add_argument('--branch', required=True, help='New poc/... branch')
            cmd.add_argument('--agent', choices=['codex', 'pi'], required=True)
            cmd.add_argument('--model', required=True)
            cmd.add_argument('--prompt', required=True)
            cmd.add_argument('--message', default='feat: apply remote agent changes')
            cmd.add_argument('--author-name', required=True)
            cmd.add_argument('--author-email', required=True)
            cmd.add_argument('--timeout', type=int, default=900)
            cmd.add_argument('--push', action='store_true')
    return root


def main():
    args = parser().parse_args()
    if args.command == 'login':
        if args.provider == 'github':
            auth.github(args.client_id)
        else:
            getattr(auth, args.provider)()
    elif args.command == 'status':
        for provider in ['github', 'openrouter']:
            path = auth.state_dir() / (provider + '.json')
            print(provider + ':', 'credential saved (not verified)' if path.exists() else 'not connected')
        print('openai:', 'credential saved (not verified)' if (auth.state_dir() / 'codex/auth.json').exists() else 'not connected')
    elif args.command == 'bootstrap':
        remote.bootstrap(args)
    else:
        gh = auth.load('github')
        if gh.get('expires_at') and gh['expires_at'] <= time.time():
            raise RuntimeError('GitHub login expired; sign in again')
        payload = vars(args).copy()
        payload['github_token'] = gh['access_token']
        if args.agent == 'codex':
            payload['codex_auth'] = json.loads((auth.state_dir() / 'codex/auth.json').read_text())
        else:
            payload['openrouter_key'] = auth.load('openrouter')['key']
        remote.launch(args, payload)


if __name__ == '__main__':
    try:
        main()
    except urllib.error.HTTPError as error:
        raise SystemExit(f'Provider HTTP error {error.code}; response body withheld to protect credentials')
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error))
