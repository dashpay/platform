import { expect } from 'chai';
import fs from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(currentDir, '../../../../..');
const stateBackupScript = path.join(repositoryRoot, 'scripts/state_backup.sh');
const volumeRestoreScript = path.join(
  repositoryRoot,
  'scripts/dashmate/volumes/restore.sh',
);

function writeExecutable(filePath, contents) {
  fs.writeFileSync(filePath, contents, { mode: 0o700 });
}

describe('operator archive scripts', () => {
  let testRoot;
  let binDir;
  let dockerLog;
  let environment;

  beforeEach(() => {
    testRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-script-test-'));
    binDir = path.join(testRoot, 'bin');
    dockerLog = path.join(testRoot, 'docker.log');
    fs.mkdirSync(binDir);

    writeExecutable(path.join(binDir, 'docker'), `#!/usr/bin/env bash
set -eu
{
  echo CALL
  for argument in "$@"; do printf 'ARG=%s\\n' "$argument"; done
} >> "$DOCKER_LOG"
if [ "\${1:-}" = volume ] && [ "\${2:-}" = inspect ]; then exit 1; fi
if [ "\${1:-}" = volume ] && [ "\${2:-}" = create ] && [ "$#" -eq 2 ]; then
  echo dashmate_staging
fi
`);
    writeExecutable(path.join(binDir, 'dashmate'), `#!/usr/bin/env bash
set -eu
if [ "\${1:-}" = config ] && [ "\${2:-}" = default ]; then
  echo test
elif [ "\${1:-}" = config ] && [ "\${2:-}" = envs ]; then
  echo COMPOSE_PROJECT_NAME=dashmate_test
fi
`);

    environment = {
      ...process.env,
      PATH: `${binDir}:${process.env.PATH}`,
      DASHMATE_CMD: path.join(binDir, 'dashmate'),
      DOCKER_LOG: dockerLog,
    };
  });

  afterEach(() => {
    fs.rmSync(testRoot, { recursive: true, force: true });
  });

  function testSpecialArchiveBasename(component) {
    it(`passes a special ${component} archive basename as data`, () => {
      const archiveName = `${component};touch owned.tar.gz`;
      const archivePath = path.join(testRoot, archiveName);
      fs.writeFileSync(archivePath, 'test');

      const result = spawnSync(
        'bash',
        [stateBackupScript, 'import', component, archivePath, '--config', 'test'],
        { cwd: testRoot, env: environment, encoding: 'utf8' },
      );

      expect(result.status, result.stderr).to.equal(0);
      expect(fs.existsSync(path.join(testRoot, 'owned.tar.gz'))).to.equal(false);
      const log = fs.readFileSync(dockerLog, 'utf8');
      expect(log).to.include(`ARG=/in/${archiveName}\n`);
      expect(log.split('\n').filter((line) => line.includes('touch owned')))
        .to.deep.equal([`ARG=/in/${archiveName}`]);
    });
  }

  testSpecialArchiveBasename('abci');
  testSpecialArchiveBasename('tenderdash');

  it('keeps imported Docker labels as single arguments', () => {
    const dumpDir = path.join(testRoot, 'dashmate_volumes_dump');
    fs.mkdirSync(dumpDir);
    fs.writeFileSync(path.join(dumpDir, 'data.tar.gz'), 'test');
    fs.writeFileSync(path.join(dumpDir, 'metadata.json'), JSON.stringify([{
      Name: 'dashmate_fixture',
      Labels: {
        origin: 'trusted --driver local --opt device=/host',
      },
    }]));

    const result = spawnSync('bash', [volumeRestoreScript], {
      cwd: testRoot,
      env: environment,
      encoding: 'utf8',
    });

    expect(result.status, result.stderr).to.equal(0);
    const log = fs.readFileSync(dockerLog, 'utf8');
    expect(log).to.include('ARG=origin=trusted --driver local --opt device=/host\n');
    expect(log).to.not.include('\nARG=--driver\n');
    expect(log).to.not.include('\nARG=--opt\n');
  });

  it('rejects a backup volume name outside the managed namespace', () => {
    const dumpDir = path.join(testRoot, 'dashmate_volumes_dump');
    fs.mkdirSync(dumpDir);
    fs.writeFileSync(path.join(dumpDir, 'data.tar.gz'), 'test');
    fs.writeFileSync(path.join(dumpDir, 'metadata.json'), JSON.stringify([{
      Name: 'foreign_volume',
      Labels: {},
    }]));

    const result = spawnSync('bash', [volumeRestoreScript], {
      cwd: testRoot,
      env: environment,
      encoding: 'utf8',
    });

    expect(result.status).to.not.equal(0);
    expect(result.stderr).to.include('invalid backup volume name');
    expect(fs.existsSync(dockerLog)).to.equal(false);
  });
});
