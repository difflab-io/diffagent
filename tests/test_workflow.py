import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from orchestration import auth, remote, worker


class WorkflowTests(unittest.TestCase):
    def test_credentials_private(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(os.environ, {'ORCH_STATE_DIR': tmp}):
            auth.save('github', {'access_token': 'fake-secret'})
            self.assertEqual(auth.load('github')['access_token'], 'fake-secret')
            self.assertEqual(Path(tmp, 'github.json').stat().st_mode & 0o777, 0o600)

    def test_agent_paths(self):
        self.assertIn('workspace-write', worker.agent_command('codex', 'model', 'hello'))
        command = worker.agent_command('pi', 'model', 'hello')
        self.assertIn('openrouter', command)
        self.assertNotIn('--api-key', command)

    def test_github_device_flow_slow_down_and_identity(self):
        responses = [dict(verification_uri='https://github.com/login/device', user_code='TEST',
                          device_code='private-code', expires_in=900, interval=5),
                     {'error': 'authorization_pending'}, {'error': 'slow_down'},
                     {'access_token': 'private-token', 'expires_in': 28800}, {'login': 'test-user'}]
        with tempfile.TemporaryDirectory() as tmp, patch.dict(os.environ, {'ORCH_STATE_DIR': tmp}), patch.object(auth, 'request', side_effect=responses) as request, patch.object(auth.time, 'sleep') as sleep, patch.object(auth.webbrowser, 'open'), contextlib.redirect_stdout(io.StringIO()) as output:
            auth.github('my-oauth-app')
            self.assertEqual(auth.load('github')['login'], 'test-user')
            self.assertEqual(sleep.call_args_list[-1].args[0], 10)
            self.assertEqual(request.call_args.kwargs['token'], 'private-token')
            self.assertNotIn('private-token', output.getvalue())

    def test_ssh_secrets_use_stdin_only(self):
        from argparse import Namespace
        args = Namespace(project='project', instance='instance', zone='zone', iap=True)
        with patch.object(remote.subprocess, 'run') as run:
            remote.launch(args, {'github_token': 'secret-for-test'})
            self.assertNotIn('secret-for-test', repr(run.call_args.args))
            self.assertEqual(json.loads(run.call_args.kwargs['input'])['github_token'], 'secret-for-test')
            self.assertIn('--tunnel-through-iap', run.call_args.args[0])

    def exercise(self, fail=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / 'source'
            source.mkdir()
            real_run = subprocess.run
            def git(*args):
                return real_run(['git', *args], cwd=source, check=True, capture_output=True, text=True)
            git('init', '-b', 'main')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.com')
            (source / 'README.md').write_text('fixture\n')
            git('add', '.')
            git('commit', '-m', 'initial')
            def intercept(command, **kwargs):
                command = list(command)
                if command[0] == 'git' and 'clone' in command:
                    command[-2] = str(source)
                return real_run(command, **kwargs)
            fake = [sys.executable, '-c', 'from pathlib import Path; Path("result.txt").write_text("done\\n")' + ('; raise SystemExit(1)' if fail else '')]
            payload = dict(repo='test/fixture', agent='pi', branch='poc/demo', base='main', model='fake',
                           prompt='Make a change', github_token='fake', openrouter_key='fake', timeout=10,
                           author_name='Test', author_email='test@example.com', message='feat: demo', push=False)
            with patch.dict(os.environ, {'HOME': tmp}), patch.object(worker.subprocess, 'run', side_effect=intercept), patch.object(worker, 'agent_command', return_value=fake), contextlib.redirect_stdout(io.StringIO()):
                if fail:
                    with self.assertRaises(subprocess.CalledProcessError):
                        worker.run(payload)
                else:
                    worker.run(payload)
            repo = next((root / 'orchestration-runs').glob('run-*/repo'))
            count = real_run(['git', 'rev-list', '--count', 'HEAD'], cwd=repo, check=True, capture_output=True, text=True).stdout.strip()
            self.assertEqual(count, '1' if fail else '2')
            self.assertTrue((repo / 'result.txt').exists())
            self.assertFalse((repo / 'auth.json').exists())

    def test_remote_runner_commits(self):
        self.exercise()

    def test_agent_failure_does_not_commit(self):
        self.exercise(fail=True)

    def test_reject_main(self):
        with self.assertRaises(ValueError):
            worker.validate({'repo': 'owner/repo', 'agent': 'codex', 'branch': 'main'})


if __name__ == '__main__':
    unittest.main()
