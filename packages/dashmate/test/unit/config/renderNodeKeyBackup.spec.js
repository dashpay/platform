import fs from 'fs';
import os from 'os';
import path from 'path';
import { spawnSync } from 'child_process';
import renderNodeKeyBackup from '../../../src/config/renderNodeKeyBackup.js';

// The backup is the only copy of a key the masternode's registration names, so
// the printed command is run here, in sh, as an operator would paste it.
describe('renderNodeKeyBackup', () => {
  let workDir;
  let binDir;

  /**
   * Run a printed command in sh, with a dashmate that prints a key
   *
   * @param {string} command
   * @param {Object} [options]
   * @param {string} [options.umask] - the operator's umask
   * @return {{ status: number, stderr: string }}
   */
  function runInShell(command, { umask = '022' } = {}) {
    const { status, stderr } = spawnSync('sh', ['-c', `umask ${umask}; ${command}`], {
      cwd: workDir,
      env: { ...process.env, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
    });

    return { status, stderr };
  }

  beforeEach(() => {
    workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-node-key-backup-'));
    binDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-node-key-bin-'));

    // Prints its arguments, so a test sees which config the backup reads.
    fs.writeFileSync(path.join(binDir, 'dashmate'), '#!/bin/sh\necho "key of $*"\n', { mode: 0o755 });
  });

  afterEach(() => {
    fs.rmSync(workDir, { recursive: true, force: true });
    fs.rmSync(binDir, { recursive: true, force: true });
  });

  // Backups of several nodes in one directory must not replace each other.
  it('should name the file after the config', () => {
    const { fileName } = renderNodeKeyBackup('testnet');

    expect(fileName).to.include('testnet');
    expect(renderNodeKeyBackup('mainnet').fileName).to.not.equal(fileName);
  });

  it('should write the key into a new file only its owner can read', () => {
    const { fileName, backupCommand } = renderNodeKeyBackup('testnet');

    const { status } = runInShell(backupCommand, { umask: '000' });

    expect(status).to.equal(0);
    const filePath = path.join(workDir, fileName);
    expect(fs.readFileSync(filePath, 'utf8'))
      .to.equal('key of config get platform.drive.tenderdash.node.key --config testnet\n');
    // eslint-disable-next-line no-bitwise
    expect(fs.statSync(filePath).mode & 0o777).to.equal(0o600);
  });

  // Truncating it would lose the earlier backup, even if the export then
  // failed, and an existing file keeps whatever mode it had.
  it('should refuse to overwrite a file that exists, and leave it as it was', () => {
    const { fileName, backupCommand } = renderNodeKeyBackup('testnet');
    const filePath = path.join(workDir, fileName);
    fs.writeFileSync(filePath, 'earlier backup\n', { mode: 0o644 });

    const { status } = runInShell(backupCommand);

    expect(status).to.not.equal(0);
    expect(fs.readFileSync(filePath, 'utf8')).to.equal('earlier backup\n');
  });

  // The umask is the operator's: left at 077, the reset that follows would
  // render files the Platform containers cannot read.
  it('should leave the operator\'s umask as it was', () => {
    const { backupCommand } = renderNodeKeyBackup('testnet');

    const { stderr } = runInShell(`${backupCommand}; umask >&2`);

    expect(stderr.trim()).to.equal('0022');
  });

  it('should keep a config name that the shell would split or expand as one argument', () => {
    const name = "it's $HOME";
    const { fileName, backupCommand, setupCommand } = renderNodeKeyBackup(name);

    const { status } = runInShell(backupCommand);

    expect(status).to.equal(0);
    expect(fs.readdirSync(workDir)).to.have.lengthOf(1);
    const [writtenName] = fs.readdirSync(workDir);
    expect(fs.readFileSync(path.join(workDir, writtenName), 'utf8'))
      .to.equal(`key of config get platform.drive.tenderdash.node.key --config ${name}\n`);
    expect(writtenName).to.not.include('/');
    expect(setupCommand).to.include(fileName);
  });

  it('should give setup the same file', () => {
    const { fileName, setupCommand } = renderNodeKeyBackup('testnet');

    expect(setupCommand).to.equal(`dashmate setup --platform --config testnet --platform-node-key-file ${fileName}`);
  });
});
