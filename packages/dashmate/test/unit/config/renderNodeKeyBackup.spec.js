import fs from 'fs';
import os from 'os';
import path from 'path';
import { spawnSync } from 'child_process';
import hasbin from 'hasbin';
import renderNodeKeyBackup from '../../../src/config/renderNodeKeyBackup.js';

// The backup is the only copy of a key the masternode's registration names, so
// the printed command is run here, in sh, as an operator would paste it.
describe('renderNodeKeyBackup', () => {
  let homeDir;
  let workDir;
  let binDir;

  /**
   * Run a printed command in a shell, with a dashmate that prints a key
   *
   * @param {string} command
   * @param {Object} [options]
   * @param {string} [options.umask] - the operator's umask
   * @param {string} [options.cwd] - where the operator runs it
   * @param {string} [options.shell]
   * @return {{ status: number, stdout: string, stderr: string }}
   */
  function runInShell(command, { umask = '022', cwd = workDir, shell = 'sh' } = {}) {
    const { status, stdout, stderr } = spawnSync(shell, ['-c', `umask ${umask}; ${command}`], {
      cwd,
      env: { ...process.env, HOME: homeDir, PATH: `${binDir}:${process.env.PATH}` },
      encoding: 'utf8',
      // A write into a FIFO without a reader would wait forever.
      timeout: 10000,
    });

    return { status, stdout, stderr };
  }

  /**
   * @param {string} filePath
   * @return {number}
   */
  function modeOf(filePath) {
    // eslint-disable-next-line no-bitwise
    return fs.statSync(filePath).mode & 0o777;
  }

  beforeEach(() => {
    homeDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-node-key-home-'));
    workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-node-key-work-'));
    binDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-node-key-bin-'));

    // Prints its arguments, so a test sees which config the backup reads.
    fs.writeFileSync(path.join(binDir, 'dashmate'), '#!/bin/sh\necho "key of $*"\n', { mode: 0o755 });
  });

  afterEach(() => {
    [homeDir, workDir, binDir].forEach((dir) => fs.rmSync(dir, { recursive: true, force: true }));
  });

  // Backups of several nodes must not replace each other.
  it('should name the backup after the config', () => {
    const { filePath } = renderNodeKeyBackup('testnet');

    expect(filePath).to.include('testnet');
    expect(renderNodeKeyBackup('mainnet').filePath).to.not.equal(filePath);
  });

  // The printed commands are pasted into whatever shell the operator has,
  // dash (Debian's and Ubuntu's /bin/sh) included.
  ['sh', 'dash', 'bash', 'zsh'].forEach((shell) => {
    it(`should write the key into a new directory only its owner can open, in ${shell}`, function it() {
      if (!hasbin.sync(shell)) {
        this.skip();
      }

      const { backupCommand } = renderNodeKeyBackup('testnet');

      const { status } = runInShell(backupCommand, { umask: '000', shell });

      expect(status).to.equal(0);
      const dirPath = path.join(homeDir, 'node-key-testnet');
      const filePath = path.join(dirPath, 'node-key');
      expect(fs.readFileSync(filePath, 'utf8'))
        .to.equal('key of config get platform.drive.tenderdash.node.key --config testnet\n');
      expect(modeOf(dirPath)).to.equal(0o700);
      expect(modeOf(filePath)).to.equal(0o600);
    });
  });

  // The reset removes the config's Platform directory, which may be where
  // the operator is, and setup may be run from somewhere else.
  it('should keep the backup in the home directory, wherever it is run, and give setup that file', () => {
    const { backupCommand, setupCommand } = renderNodeKeyBackup('testnet');

    expect(runInShell(backupCommand).status).to.equal(0);

    expect(fs.readdirSync(workDir)).to.be.empty();
    expect(setupCommand).to.equal('dashmate setup --platform --config testnet'
      + ' --platform-node-key-file "$HOME"/node-key-testnet/node-key');
    const { filePath } = renderNodeKeyBackup('testnet');
    expect(runInShell(`cat ${filePath}`, { cwd: binDir }).stdout)
      .to.equal('key of config get platform.drive.tenderdash.node.key --config testnet\n');
  });

  // The shell's noclobber refuses only a regular file: a FIFO there would
  // hand the key to whoever reads it.
  it('should refuse a FIFO where the backup goes, writing nothing into it', function it() {
    if (!hasbin.sync('mkfifo')) {
      this.skip();
    }

    const fifoPath = path.join(homeDir, 'node-key-testnet');
    expect(spawnSync('mkfifo', [fifoPath]).status).to.equal(0);
    // eslint-disable-next-line no-bitwise
    const fd = fs.openSync(fifoPath, fs.constants.O_RDONLY | fs.constants.O_NONBLOCK);

    try {
      const { backupCommand } = renderNodeKeyBackup('testnet');

      const { status } = runInShell(backupCommand, { cwd: homeDir });

      let bytesRead = 0;
      try {
        bytesRead = fs.readSync(fd, Buffer.alloc(1024));
      } catch (e) {
        if (e.code !== 'EAGAIN') {
          throw e;
        }
      }

      expect(bytesRead).to.equal(0);
      expect(status).to.not.equal(0);
      expect(fs.lstatSync(fifoPath).isFIFO()).to.be.true();
    } finally {
      fs.closeSync(fd);
    }
  });

  // Writing into it would lose the earlier backup, even if the export then
  // failed, and an existing file keeps whatever mode it had.
  it('should refuse a backup that exists, and leave it as it was', () => {
    const { backupCommand } = renderNodeKeyBackup('testnet');
    const dirPath = path.join(homeDir, 'node-key-testnet');
    fs.mkdirSync(dirPath, { mode: 0o755 });
    fs.writeFileSync(path.join(dirPath, 'node-key'), 'earlier backup\n', { mode: 0o644 });

    const { status } = runInShell(backupCommand);

    expect(status).to.not.equal(0);
    expect(fs.readFileSync(path.join(dirPath, 'node-key'), 'utf8')).to.equal('earlier backup\n');
  });

  // A failed export must leave nothing that passes for a backup, and
  // nothing that keeps the next attempt from running.
  it('should remove what a failed export wrote, so that it can run again', () => {
    const { backupCommand } = renderNodeKeyBackup('testnet');
    fs.writeFileSync(path.join(binDir, 'dashmate'), '#!/bin/sh\nprintf partial\nexit 1\n', { mode: 0o755 });

    const { status } = runInShell(backupCommand);

    expect(status).to.not.equal(0);
    expect(fs.readdirSync(homeDir)).to.be.empty();

    fs.writeFileSync(path.join(binDir, 'dashmate'), '#!/bin/sh\necho "key of $*"\n', { mode: 0o755 });

    expect(runInShell(backupCommand).status).to.equal(0);
    expect(fs.readFileSync(path.join(homeDir, 'node-key-testnet', 'node-key'), 'utf8'))
      .to.equal('key of config get platform.drive.tenderdash.node.key --config testnet\n');
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
    const { filePath, backupCommand, setupCommand } = renderNodeKeyBackup(name);

    const { status } = runInShell(backupCommand);

    expect(status).to.equal(0);
    const entries = fs.readdirSync(homeDir);
    expect(entries).to.have.lengthOf(1);
    expect(fs.readFileSync(path.join(homeDir, entries[0], 'node-key'), 'utf8'))
      .to.equal(`key of config get platform.drive.tenderdash.node.key --config ${name}\n`);
    expect(setupCommand).to.include(filePath);
  });
});
