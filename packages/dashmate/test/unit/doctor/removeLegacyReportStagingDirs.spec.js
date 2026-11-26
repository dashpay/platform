import fs from 'fs';
import os from 'os';
import path from 'path';
import {
  findLegacyReportStagingDirs,
  removeLegacyReportStagingDirs,
} from '../../../src/doctor/removeLegacyReportStagingDirs.js';

describe('removeLegacyReportStagingDirs', () => {
  /**
   * Find what earlier versions left and remove all of it, as an operator who
   * agrees to remove everything found would
   *
   * @param {string} root
   * @return {{removed: string[], failed: Object[], scanFailure: Object|null}}
   */
  function cleanUp(root) {
    const { found, left, scanFailure } = findLegacyReportStagingDirs(root);
    const { removed, failed } = removeLegacyReportStagingDirs(found);

    return { removed, failed: [...left, ...failed], scanFailure };
  }

  const legacyName = 'dashmate-report-2026-01-01T00:00:00.000Z';

  let testRoot;
  let tempDir;
  let outsideDir;

  /**
   * A directory laid out the way earlier versions staged a report
   *
   * @param {string} name
   * @param {string} [root]
   * @return {string}
   */
  function makeStagingDir(name, root = tempDir) {
    const dir = path.join(root, name);

    fs.mkdirSync(path.join(dir, 'core'), { recursive: true });
    fs.writeFileSync(path.join(dir, 'date.txt'), name.slice('dashmate-report-'.length));
    fs.writeFileSync(path.join(dir, 'systemInfo.json'), '{}');
    fs.writeFileSync(path.join(dir, 'dashmateConfig.json'), '{}');
    fs.writeFileSync(path.join(dir, 'dashmateVersion.txt'), '4.0.0');
    fs.writeFileSync(path.join(dir, 'core', 'stdOut.txt'), 'log line\n');
    fs.writeFileSync(path.join(dir, 'core', 'dockerInspect.json'), '{}');

    return dir;
  }

  /**
   * @param {string} dir
   * @return {number}
   */
  function modeOf(dir) {
    // eslint-disable-next-line no-bitwise
    return fs.lstatSync(dir).mode & 0o777;
  }

  /**
   * Make one path report something else when it is inspected without
   * following links, the way the cleanup inspects every entry
   *
   * @param {Object} sinon
   * @param {string} target
   * @param {Object} overrides
   */
  function stubLstatOf(sinon, target, overrides) {
    const lstatSync = fs.lstatSync.bind(fs);

    sinon.stub(fs, 'lstatSync').callsFake((entry, ...rest) => {
      const stat = lstatSync(entry, ...rest);

      return entry === target ? Object.assign(Object.create(stat), overrides) : stat;
    });
  }

  beforeEach(() => {
    // Resolved, so the paths the cleanup reports compare equal to these
    testRoot = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-legacy-staging-test-')));
    tempDir = path.join(testRoot, 'tmp');
    outsideDir = path.join(testRoot, 'outside');

    fs.mkdirSync(tempDir, { mode: 0o700 });
    fs.mkdirSync(outsideDir);
  });

  afterEach(() => {
    fs.chmodSync(testRoot, 0o700);
    fs.rmSync(testRoot, { recursive: true, force: true });
  });

  // Earlier versions staged every report under a fixed name in the temp
  // directory and never removed it.
  it('should remove a staging directory an earlier version left behind', () => {
    const dir = makeStagingDir(legacyName);

    const { removed, failed } = cleanUp(tempDir);

    expect(removed).to.deep.equal([dir]);
    expect(failed).to.be.empty();
    expect(fs.existsSync(dir)).to.be.false();
  });

  // What is found is the operator's to agree to remove, so finding it
  // removes nothing
  it('should find a staging directory without removing it', () => {
    const dir = makeStagingDir(legacyName);

    const { found, left, scanFailure } = findLegacyReportStagingDirs(tempDir);

    expect(found.map(({ dir: foundDir }) => foundDir)).to.deep.equal([dir]);
    expect(left).to.be.empty();
    expect(scanFailure).to.be.null();
    expect(fs.existsSync(dir)).to.be.true();
  });

  // Time passes between finding a directory and the operator agreeing to
  // remove it
  it('should leave a directory that changed after it was found', () => {
    const dir = makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    fs.writeFileSync(path.join(dir, 'notes.txt'), 'written after');

    const { removed, failed } = removeLegacyReportStagingDirs(found);

    expect(removed).to.be.empty();
    expect(failed.map(({ dir: failedDir, restricted }) => [failedDir, restricted]))
      .to.deep.equal([[dir, true]]);
    expect(fs.existsSync(path.join(dir, 'notes.txt'))).to.be.true();
  });

  // What the operator agreed to remove is the directory that was looked at,
  // not whatever is at its path when they answer
  it('should not remove a different directory put at the path of one it found', () => {
    const dir = makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    fs.renameSync(dir, path.join(testRoot, 'original'));
    makeStagingDir(legacyName);

    const { removed, failed } = removeLegacyReportStagingDirs(found);

    expect(removed).to.be.empty();
    expect(failed.map(({ dir: failedDir }) => failedDir)).to.deep.equal([dir]);
    expect(fs.existsSync(path.join(dir, 'date.txt'))).to.be.true();
  });

  // Whatever is put at the path while the directory is being removed is not
  // what was checked, so it is not what goes
  it('should remove the checked directory, not one put at its path while removing it', function it() {
    const dir = makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    const rmSync = fs.rmSync.bind(fs);
    let replaced = false;

    this.sinon.stub(fs, 'rmSync').callsFake((target, options) => {
      if (!replaced && path.basename(target) === legacyName) {
        replaced = true;

        if (fs.existsSync(dir)) {
          fs.renameSync(dir, path.join(testRoot, 'original'));
        }

        makeStagingDir(legacyName);
        fs.writeFileSync(path.join(dir, 'core', 'stdOut.txt'), 'replacement\n');
      }

      return rmSync(target, options);
    });

    const { removed } = removeLegacyReportStagingDirs(found);

    expect(removed).to.deep.equal([dir]);
    expect(fs.readFileSync(path.join(dir, 'core', 'stdOut.txt'), 'utf8')).to.equal('replacement\n');
  });

  // Putting it back could replace whatever was put at its path meanwhile, so
  // it stays where it was moved, closed to others, and the operator is told
  it('should leave what was moved for removal where it is, and say where, if it is not the one found', function it() {
    const dir = makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    const lstatSync = fs.lstatSync.bind(fs);

    this.sinon.stub(fs, 'lstatSync').callsFake((entry, ...rest) => {
      const stat = lstatSync(entry, ...rest);

      return String(entry).includes('.dashmate-report-removal-') && path.basename(entry) === legacyName
        ? Object.assign(Object.create(stat), { ino: stat.ino + 1 })
        : stat;
    });

    const { removed, failed } = removeLegacyReportStagingDirs(found);

    const [holdingName] = fs.readdirSync(tempDir);
    const moved = path.join(tempDir, holdingName, legacyName);

    expect(removed).to.be.empty();
    expect(fs.readdirSync(tempDir)).to.deep.equal([holdingName]);
    expect(holdingName).to.match(/^\.dashmate-report-removal-/);
    expect(modeOf(path.join(tempDir, holdingName))).to.equal(0o700);
    expect(fs.existsSync(path.join(moved, 'date.txt'))).to.be.true();
    expect(failed.map(({ dir: failedDir, restricted }) => [failedDir, restricted]))
      .to.deep.equal([[dir, true]]);
    expect(failed[0].error.message).to.equal(
      `${dir} is not the directory that was found there; it is now in ${moved}`,
    );
  });

  it('should remove its empty holding directory and report why when it cannot move a directory there', function it() {
    const dir = makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    const renameError = Object.assign(new Error('no such file or directory'), { code: 'ENOENT' });

    this.sinon.stub(fs, 'renameSync').throws(renameError);

    const { removed, failed } = removeLegacyReportStagingDirs(found);

    expect(removed).to.be.empty();
    expect(failed.map(({ dir: failedDir, error }) => [failedDir, error])).to.deep.equal([[dir, renameError]]);
    expect(fs.readdirSync(tempDir)).to.deep.equal([legacyName]);
  });

  // What a removal that stopped partway left can be anything by now, so it is
  // pointed out for the operator to look at, and never touched
  it('should list the holding directories of removals that did not finish without touching them', () => {
    const holding = path.join(tempDir, '.dashmate-report-removal-Ab12Cd');
    fs.mkdirSync(holding, { mode: 0o700 });
    const moved = makeStagingDir(legacyName, holding);

    fs.symlinkSync(outsideDir, path.join(tempDir, '.dashmate-report-removal-link01'));
    fs.writeFileSync(path.join(tempDir, '.dashmate-report-removal-file01'), '');

    const { found, left, recovery } = findLegacyReportStagingDirs(tempDir);

    expect(found).to.be.empty();
    expect(left).to.be.empty();
    expect(recovery).to.deep.equal([{ dir: holding }]);

    expect(removeLegacyReportStagingDirs(found).removed).to.be.empty();
    expect(fs.existsSync(path.join(moved, 'date.txt'))).to.be.true();
    expect(modeOf(holding)).to.equal(0o700);
  });

  it('should leave nothing of its own behind once a directory is removed', () => {
    makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    removeLegacyReportStagingDirs(found);

    expect(fs.readdirSync(tempDir)).to.be.empty();
  });

  // The directory was found in one temp directory; by the time the operator
  // agrees, that path can lead somewhere else
  it('should not remove anything once the temp directory is no longer the one it was found in', () => {
    makeStagingDir(legacyName);

    const { found } = findLegacyReportStagingDirs(tempDir);

    const elsewhere = path.join(testRoot, 'elsewhere');
    fs.mkdirSync(elsewhere, { mode: 0o700 });
    const other = makeStagingDir(legacyName, elsewhere);

    fs.renameSync(tempDir, path.join(testRoot, 'moved'));
    fs.symlinkSync(elsewhere, tempDir);

    const { removed, failed } = removeLegacyReportStagingDirs(found);

    expect(removed).to.be.empty();
    expect(failed).to.have.lengthOf(1);
    expect(fs.existsSync(other)).to.be.true();
  });

  // A record of any other size is not one an earlier version wrote, and
  // whatever it holds instead is not read
  it('should not read a collection date of any other size than a record', function it() {
    const dir = makeStagingDir(legacyName);

    fs.writeFileSync(path.join(dir, 'date.txt'), 'x'.repeat(1024 * 1024));

    const readFileSync = this.sinon.spy(fs, 'readFileSync');
    const readSync = this.sinon.spy(fs, 'readSync');

    const { found, left } = findLegacyReportStagingDirs(tempDir);

    expect(found).to.be.empty();
    expect(left.map(({ dir: leftDir }) => leftDir)).to.deep.equal([dir]);
    expect(readFileSync).to.not.have.been.calledWith(path.join(dir, 'date.txt'));
    expect(readSync).to.not.have.been.called();
  });

  it('should leave a link under that name, and what it points to, alone', () => {
    fs.writeFileSync(path.join(outsideDir, 'keep.txt'), 'not ours to remove');

    const link = path.join(tempDir, legacyName);
    fs.symlinkSync(outsideDir, link);

    const { removed } = cleanUp(tempDir);

    expect(removed).to.be.empty();
    expect(fs.lstatSync(link).isSymbolicLink()).to.be.true();
    expect(fs.readFileSync(path.join(outsideDir, 'keep.txt'), 'utf8')).to.equal('not ours to remove');
  });

  it('should leave a directory another user owns alone', function it() {
    const dir = makeStagingDir(legacyName);

    stubLstatOf(this.sinon, dir, { uid: process.getuid() + 1 });

    const { removed, failed } = cleanUp(tempDir);

    expect(removed).to.be.empty();
    // Passed over, not refused: it is simply not one of the user's own
    expect(failed).to.be.empty();
    expect(fs.existsSync(dir)).to.be.true();
  });

  it('should leave a directory alone that is on another file system than the temp directory', function it() {
    const dir = makeStagingDir(legacyName);

    // The staging directory and all it holds on one file system, mounted
    // where the temp directory is on another
    stubLstatOf(this.sinon, tempDir, { dev: fs.lstatSync(tempDir).dev + 1 });

    const { removed, failed } = cleanUp(tempDir);

    expect(removed).to.be.empty();
    expect(failed.map(({ dir: failedDir }) => failedDir)).to.deep.equal([dir]);
    expect(fs.existsSync(dir)).to.be.true();
  });

  // The current version stages in a directory of its own with a random
  // suffix, and a report being written right now must not lose its files.
  it('should only remove names that match the earlier naming exactly', () => {
    const kept = [
      makeStagingDir(`${legacyName}-a1B2c3`),
      makeStagingDir('dashmate-report-latest'),
      makeStagingDir(`x${legacyName}`),
      // Shaped like the earlier name, but no collection time is written this way
      makeStagingDir('dashmate-report-2026-99-01T00:00:00.000Z'),
    ];

    const file = path.join(tempDir, 'dashmate-report-2026-01-02T00:00:00.000Z');
    fs.writeFileSync(file, 'a file, not a staging directory');

    const { removed } = cleanUp(tempDir);

    expect(removed).to.be.empty();
    kept.forEach((dir) => expect(fs.existsSync(dir)).to.be.true());
    expect(fs.existsSync(file)).to.be.true();
  });

  // Where the operating system has no user ids to compare, ownership cannot
  // be told, so nothing is looked at. Earlier versions could not create these
  // directories there in the first place: the name is not a valid one.
  it('should do nothing where ownership cannot be told', function it() {
    this.sinon.stub(process, 'getuid').value(undefined);
    const readdirSync = this.sinon.spy(fs, 'readdirSync');

    const { removed, failed } = cleanUp(tempDir);

    expect(removed).to.be.empty();
    expect(failed).to.be.empty();
    expect(readdirSync).to.not.have.been.called();
  });

  it('should report a directory it could not remove and carry on', function it() {
    const first = makeStagingDir(legacyName);
    const second = makeStagingDir('dashmate-report-2026-01-03T00:00:00.000Z');

    const rmSync = fs.rmSync.bind(fs);
    this.sinon.stub(fs, 'rmSync').callsFake((target, options) => {
      if (path.basename(target) === path.basename(first)) {
        throw new Error('permission denied');
      }

      return rmSync(target, options);
    });

    const { removed, failed } = cleanUp(tempDir);

    expect(removed).to.deep.equal([second]);
    expect(failed.map(({ dir }) => dir)).to.deep.equal([first]);
    expect(failed[0].error.message).to.match(/^permission denied; what is left of it is in /);
  });

  // What is removed is decided by name and owner, and both only hold while
  // nobody else can swap the directory being looked at for another one.
  describe('a temp directory others could change', () => {
    it('should leave everything alone in a directory anyone can rename entries in', () => {
      const dir = makeStagingDir(legacyName);

      fs.chmodSync(tempDir, 0o777);

      const { removed, failed, scanFailure } = cleanUp(tempDir);

      expect(removed).to.be.empty();
      expect(failed).to.be.empty();
      expect(scanFailure.dir).to.equal(tempDir);
      expect(fs.existsSync(dir)).to.be.true();
    });

    // That is what a shared temp directory is like: anyone can add entries,
    // but only their owner can rename or remove them
    it('should remove from a directory where entries can only be renamed by their owner', () => {
      const dir = makeStagingDir(legacyName);

      fs.chmodSync(tempDir, 0o1777);

      const { removed } = cleanUp(tempDir);

      expect(removed).to.deep.equal([dir]);
    });

    it('should leave everything alone when a directory above it can be changed by others', () => {
      const dir = makeStagingDir(legacyName);

      fs.chmodSync(testRoot, 0o777);

      const { removed, failed, scanFailure } = cleanUp(tempDir);

      expect(removed).to.be.empty();
      expect(failed).to.be.empty();
      expect(scanFailure).to.not.be.null();
      expect(fs.existsSync(dir)).to.be.true();
    });

    // Through a link, `..` leads the operating system to the link target's
    // parent, while resolving the path as text drops the link instead, so the
    // same path names two directories. Nothing is removed from either.
    it('should leave everything alone when the path to it is not a plain one', () => {
      fs.symlinkSync(tempDir, path.join(outsideDir, 'link'));

      const dir = makeStagingDir(legacyName);
      const other = makeStagingDir(legacyName, path.join(outsideDir, 'tmp'));

      const { removed, failed, scanFailure } = cleanUp(`${outsideDir}/link/../tmp`);

      expect(removed).to.be.empty();
      expect(failed).to.be.empty();
      expect(scanFailure).to.not.be.null();
      expect(fs.existsSync(dir)).to.be.true();
      expect(fs.existsSync(other)).to.be.true();
    });

    it('should work on the directory a link to it leads to', () => {
      const link = path.join(outsideDir, 'link');
      fs.symlinkSync(tempDir, link);

      const dir = makeStagingDir(legacyName);

      const { removed } = cleanUp(link);

      expect(removed).to.deep.equal([dir]);
    });
  });

  // A directory tree is removed only once nobody else can change it while it
  // is being removed, and only if all of it is on the same file system.
  describe('what a staging directory holds', () => {
    // The same name is what unpacking an earlier report next to its archive
    // produces, and that copy can hold notes of the operator's. Only what an
    // earlier version wrote while staging is removed.
    [
      ['a file of its own at the top', (dir) => fs.writeFileSync(path.join(dir, 'notes.txt'), 'mine')],
      ['a file of its own beside the samples', (dir) => fs.writeFileSync(path.join(dir, 'core', 'notes.txt'), 'mine')],
      ['a directory deeper than the samples', (dir) => fs.mkdirSync(path.join(dir, 'core', 'more'))],
      ['a collection date other than its name', (dir) => fs.writeFileSync(path.join(dir, 'date.txt'), 'edited')],
      ['no collection date', (dir) => fs.rmSync(path.join(dir, 'date.txt'))],
      ['a link among the samples', (dir) => fs.symlinkSync(outsideDir, path.join(dir, 'core', 'peers.json'))],
      ['a directory no service had', (dir) => {
        fs.mkdirSync(path.join(dir, 'scratch'));
        fs.writeFileSync(path.join(dir, 'scratch', 'status.txt'), 'mine');
      }],
      ['a sample of one service under another', (dir) => fs.writeFileSync(path.join(dir, 'core', 'genesis.json'), '{}')],
    ].forEach(([description, change]) => {
      it(`should leave a directory with ${description} in place and closed to others`, () => {
        const dir = makeStagingDir(legacyName);

        change(dir);

        const { removed, failed } = cleanUp(tempDir);

        expect(removed).to.be.empty();
        expect(failed.map(({ dir: failedDir, restricted }) => [failedDir, restricted]))
          .to.deep.equal([[dir, true]]);
        expect(fs.existsSync(dir)).to.be.true();
        expect(modeOf(dir)).to.equal(0o700);
      });
    });

    it('should remove a tree that others could write to, once only its owner can', function it() {
      const dir = makeStagingDir(legacyName);
      const writable = path.join(dir, 'core');

      fs.chmodSync(writable, 0o777);

      const openSync = this.sinon.spy(fs, 'openSync');
      const fchmodSync = this.sinon.spy(fs, 'fchmodSync');
      const readdirSync = this.sinon.spy(fs, 'readdirSync');

      const { removed } = cleanUp(tempDir);

      expect(removed).to.deep.equal([dir]);

      // Closed to others before what it holds is read, through the directory
      // itself rather than its path, so a link put in its place is not followed
      const opened = openSync.withArgs(writable).firstCall;
      const closed = fchmodSync.getCalls().find((call) => call.calledAfter(opened));

      // eslint-disable-next-line no-bitwise
      expect(opened.args[1] & fs.constants.O_NOFOLLOW).to.not.equal(0);
      expect(closed.args).to.deep.equal([opened.returnValue, 0o700]);
      expect(closed).to.have.been.calledBefore(readdirSync.withArgs(writable).firstCall);
    });

    // What it cannot remove it still keeps from other users: the samples in
    // it are what the report masks
    // Its owner can still write to it, so it can still hold samples
    it('should close and remove a staging directory its owner could not read', () => {
      const dir = makeStagingDir(legacyName);

      fs.chmodSync(dir, 0o355);

      const { removed } = cleanUp(tempDir);

      expect(removed).to.deep.equal([dir]);
    });

    it('should leave a tree another user owns part of, closed to others', function it() {
      const dir = makeStagingDir(legacyName);
      const entry = path.join(dir, 'core', 'stdOut.txt');

      fs.chmodSync(dir, 0o755);
      fs.chmodSync(path.join(dir, 'core'), 0o755);

      stubLstatOf(this.sinon, entry, { uid: process.getuid() + 1 });

      const { removed, failed } = cleanUp(tempDir);

      expect(removed).to.be.empty();
      expect(failed.map(({ dir: failedDir, restricted }) => [failedDir, restricted]))
        .to.deep.equal([[dir, true]]);
      expect(fs.existsSync(entry)).to.be.true();
      expect(modeOf(dir)).to.equal(0o700);
      expect(modeOf(path.join(dir, 'core'))).to.equal(0o700);
    });

    it('should leave a tree alone if part of it is on another file system', function it() {
      const dir = makeStagingDir(legacyName);
      const mountPoint = path.join(dir, 'core');

      stubLstatOf(this.sinon, mountPoint, { dev: fs.lstatSync(dir).dev + 1 });

      const { removed, failed } = cleanUp(tempDir);

      expect(removed).to.be.empty();
      expect(failed.map(({ dir: failedDir }) => failedDir)).to.deep.equal([dir]);
      expect(fs.existsSync(path.join(mountPoint, 'stdOut.txt'))).to.be.true();
    });
  });
});
