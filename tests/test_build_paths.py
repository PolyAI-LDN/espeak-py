import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class BuildPathTests(unittest.TestCase):
    def test_build_from_checkout_with_spaces(self):
        source = Path(__file__).resolve().parents[1] / 'tools' / 'build.sh'
        with tempfile.TemporaryDirectory() as temporary:
            checkout = Path(temporary) / 'checkout with spaces'
            tools = checkout / 'tools'
            native = checkout / 'espeak-ng'
            binaries = Path(temporary) / 'bin'
            for directory in (tools, native, binaries):
                directory.mkdir(parents=True)
            (tools / 'build.sh').write_text(source.read_text())
            for command in ('autogen.sh', 'configure'):
                path = native / command
                path.write_text('#!/bin/sh\npwd >> "$BUILD_LOG"\n')
                path.chmod(0o755)
            for command, text in (
                ('make', '#!/bin/sh\npwd >> "$BUILD_LOG"\n'),
                ('uname', '#!/bin/sh\nprintf "Linux\\n"\n'),
            ):
                path = binaries / command
                path.write_text(text)
                path.chmod(0o755)
            log = Path(temporary) / 'commands.log'
            environment = dict(os.environ, BUILD_LOG=str(log))
            environment['PATH'] = str(binaries) + os.pathsep + environment['PATH']
            result = subprocess.run(
                [os.environ.get('BASH', 'bash'), str(tools / 'build.sh')],
                env=environment, text=True, capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            commands = log.read_text().splitlines()
            self.assertEqual(len(commands), 3)
            self.assertTrue(all(Path(path).resolve() == native.resolve() for path in commands))
