import { expect } from 'chai';
import fs from 'fs';
import os from 'os';
import path from 'path';
import { create } from 'tar';
import unarchiveSamplesFactory from '../../../src/doctor/unarchiveSamplesFactory.js';

describe('unarchiveSamplesFactory', () => {
  let testRoot;
  let sourceDir;
  let previousTmpDir;

  beforeEach(() => {
    testRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-unarchive-test-'));
    sourceDir = path.join(testRoot, 'source');
    fs.mkdirSync(sourceDir);
    previousTmpDir = process.env.TMPDIR;
    process.env.TMPDIR = testRoot;
  });

  afterEach(() => {
    if (previousTmpDir === undefined) {
      delete process.env.TMPDIR;
    } else {
      process.env.TMPDIR = previousTmpDir;
    }
    fs.rmSync(testRoot, { recursive: true, force: true });
  });

  it('extracts samples in a private directory and cleans it up', async () => {
    fs.writeFileSync(path.join(sourceDir, 'date.txt'), '2026-01-01T00:00:00.000Z');
    fs.writeFileSync(path.join(sourceDir, 'systemInfo.json'), '{"os":"test"}');
    fs.writeFileSync(path.join(sourceDir, 'dashmateVersion.txt'), '4.0.0');
    const archivePath = path.join(testRoot, '.tar.gz');
    await create({ cwd: sourceDir, gzip: true, file: archivePath }, ['.']);

    const unarchiveSamples = unarchiveSamplesFactory(() => []);
    const samples = await unarchiveSamples(archivePath);

    expect(samples.getSystemInfo()).to.deep.equal({ os: 'test' });
    expect(samples.getDashmateVersion()).to.equal('4.0.0');
    expect(fs.readdirSync(testRoot).filter((name) => name.startsWith('dashmate-doctor-')))
      .to.deep.equal([]);
  });

  it('rejects symbolic-link archive members', async () => {
    fs.writeFileSync(path.join(sourceDir, 'target.txt'), 'target');
    fs.symlinkSync('target.txt', path.join(sourceDir, 'linked.txt'));
    const archivePath = path.join(testRoot, 'linked.tar.gz');
    await create({ cwd: sourceDir, gzip: true, file: archivePath }, ['.']);

    const unarchiveSamples = unarchiveSamplesFactory(() => []);

    await expect(unarchiveSamples(archivePath)).to.be.rejectedWith(
      'Unsupported diagnostic archive entry type',
    );
    expect(fs.readdirSync(testRoot).filter((name) => name.startsWith('dashmate-doctor-')))
      .to.deep.equal([]);
  });
});
