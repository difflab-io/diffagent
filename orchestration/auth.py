import base64
import hashlib
import http.server
import json
import os
from pathlib import Path
import secrets
import subprocess
import time
import urllib.parse
import urllib.request
import webbrowser


def state_dir():
    path = Path(os.environ.get('ORCH_STATE_DIR', Path.home() / '.local/state/orchestration'))
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    path.chmod(0o700)
    return path


def save(provider, value):
    path = state_dir() / (provider + '.json')
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(value, stream)
    path.chmod(0o600)


def load(provider):
    return json.loads((state_dir() / (provider + '.json')).read_text())


def request(url, data=None, token=None):
    headers = {'Accept': 'application/json', 'User-Agent': 'orchestration-poc'}
    if token:
        headers['Authorization'] = 'Bearer ' + token
    if data is not None:
        headers['Content-Type'] = 'application/json'
    req = urllib.request.Request(url, data=json.dumps(data).encode() if data is not None else None, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)


def github(client_id):
    if not client_id:
        subprocess.run(['gh', 'auth', 'login', '--hostname', 'github.com', '--web', '--git-protocol', 'https'], check=True)
        token = subprocess.check_output(['gh', 'auth', 'token', '--hostname', 'github.com'], text=True).strip()
        result = {'access_token': token}
    else:
        codes = request('https://github.com/login/device/code', {'client_id': client_id, 'scope': 'repo read:user'})
        print('Open', codes['verification_uri'], 'and enter', codes['user_code'], flush=True)
        webbrowser.open(codes['verification_uri'])
        deadline = time.monotonic() + codes['expires_in']
        interval = codes['interval']
        while time.monotonic() < deadline:
            time.sleep(interval)
            result = request('https://github.com/login/oauth/access_token', {
                'client_id': client_id, 'device_code': codes['device_code'],
                'grant_type': 'urn:ietf:params:oauth:grant-type:device_code'})
            if 'access_token' in result:
                break
            error = result.get('error')
            if error == 'slow_down':
                interval += 5
            elif error != 'authorization_pending':
                raise RuntimeError('GitHub authorization failed: ' + str(error))
        else:
            raise RuntimeError('GitHub authorization expired')
    user = request('https://api.github.com/user', token=result['access_token'])
    result['login'] = user['login']
    result['client_id'] = client_id
    result['expires_at'] = time.time() + result['expires_in'] if result.get('expires_in') else None
    save('github', result)
    print('Signed in to GitHub as', user['login'])


def openrouter():
    verifier = secrets.token_urlsafe(48)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).decode().rstrip('=')
    callback_path = '/callback/' + secrets.token_urlsafe(24)
    result = {}

    class Callback(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            parsed = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed.query)
            if parsed.path != callback_path or not params.get('code'):
                self.send_error(400)
                return
            result['code'] = params['code'][0]
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'Authorization received. Return to your terminal.')

        def log_message(self, *args):
            pass  # Callback URLs contain authorization codes.

    with http.server.HTTPServer(('127.0.0.1', 0), Callback) as server:
        server.timeout = 1
        callback = f'http://localhost:{server.server_port}{callback_path}'
        url = 'https://openrouter.ai/auth?' + urllib.parse.urlencode({
            'callback_url': callback, 'code_challenge': challenge, 'code_challenge_method': 'S256'})
        print('Open', url, flush=True)
        webbrowser.open(url)
        deadline = time.monotonic() + 300
        while 'code' not in result and time.monotonic() < deadline:
            server.handle_request()
    if 'code' not in result:
        raise RuntimeError('OpenRouter authorization timed out')
    key = request('https://openrouter.ai/api/v1/auth/keys', {
        'code': result['code'], 'code_verifier': verifier, 'code_challenge_method': 'S256'})
    save('openrouter', {'key': key['key']})
    print('Connected OpenRouter (uses your API credits)')


def openai():
    home = state_dir() / 'codex'
    home.mkdir(mode=0o700, exist_ok=True)
    env = dict(os.environ, CODEX_HOME=str(home))
    subprocess.run(['codex', '-c', 'cli_auth_credentials_store="file"', 'login', '--device-auth'], env=env, check=True)
    if not (home / 'auth.json').exists():
        raise RuntimeError('Codex did not create a transferable file login')
    (home / 'auth.json').chmod(0o600)
    print('Signed in through Codex; login stays in the isolated PoC state directory')
