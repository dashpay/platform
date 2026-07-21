import { expect } from 'chai';
import fs from 'fs';
import os from 'os';
import path from 'path';
import { gzipSync } from 'zlib';
import { create, Header } from 'tar';
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

  it('aborts parsing as soon as the archive member budget is exceeded', async () => {
    const headers = Array.from({ length: 10_001 }, (_, index) => {
      const header = new Header({
        path: `entry-${index}.txt`,
        type: 'File',
        size: 0,
        mode: 0o600,
        uid: 0,
        gid: 0,
        mtime: new Date(0),
      });
      header.encode();
      return header.block;
    });
    const archivePath = path.join(testRoot, 'too-many-members.tar.gz');
    fs.writeFileSync(archivePath, gzipSync(Buffer.concat([...headers, Buffer.alloc(1024)])));

    const unarchiveSamples = unarchiveSamplesFactory(() => []);

    await expect(unarchiveSamples(archivePath)).to.be.rejectedWith(
      'Diagnostic archive exceeds extraction budget',
    );
    expect(fs.readdirSync(testRoot).filter((name) => name.startsWith('dashmate-doctor-')))
      .to.deep.equal([]);
  });
});
