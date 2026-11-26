import fs from 'fs';
import os from 'os';
import path from 'path';
import archiveSamples from '../../../src/doctor/archiveSamples.js';
import Samples from '../../../src/doctor/Samples.js';
import unarchiveSamplesFactory from '../../../src/doctor/unarchiveSamplesFactory.js';

describe('archiveSamples', () => {
  let testRoot;
  let outputDir;
  let previousTmpDir;
  let samples;

  beforeEach(() => {
    testRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-archive-test-'));
    outputDir = path.join(testRoot, 'output');
    fs.mkdirSync(outputDir);

    // The samples are staged in the temporary directory before being packed
    previousTmpDir = process.env.TMPDIR;
    process.env.TMPDIR = testRoot;

    samples = new Samples();
    samples.date = new Date('2026-11-26T10:15:30.123Z');
    samples.setDashmateVersion('5.2.0');
  });

  afterEach(() => {
    if (previousTmpDir === undefined) {
      delete process.env.TMPDIR;
    } else {
      process.env.TMPDIR = previousTmpDir;
    }

    fs.rmSync(testRoot, { recursive: true, force: true });
  });

  // Operators run doctor on Windows too, where a file name cannot contain ':'
  it('should name the archive with a timestamp that is a valid file name everywhere', async () => {
    const archiveFilePath = await archiveSamples(samples, outputDir);

    const archiveName = 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz';

    expect(fs.readdirSync(outputDir)).to.deep.equal([archiveName]);
    expect(archiveFilePath).to.equal(path.join(outputDir, archiveName));
  });

  // The file name is only a label: the collection time an analysis judges
  // certificate dates against is the one recorded inside the archive
  it('should keep the exact collection time inside the archive', async () => {
    await archiveSamples(samples, outputDir);

    const [archiveName] = fs.readdirSync(outputDir);
    const unarchived = await unarchiveSamplesFactory(() => [])(path.join(outputDir, archiveName));

    expect(unarchived.date.toISOString()).to.equal('2026-11-26T10:15:30.123Z');
  });
});
