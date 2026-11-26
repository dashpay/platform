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

  describe('when packing fails', () => {
    beforeEach(function beforeEach() {
      // Root reads a file whatever its mode, and Windows cannot revoke read
      // access with a mode, so packing cannot be made to fail there
      if (process.getuid?.() === 0 || process.platform === 'win32') {
        this.skip();
      }

      // Make one staged sample unreadable, so that packing fails after the
      // archive file has been opened
      const { writeFileSync } = fs;
      this.sinon.stub(fs, 'writeFileSync').callsFake((filePath, ...args) => {
        writeFileSync(filePath, ...args);

        if (path.basename(filePath) === 'dashmateVersion.txt') {
          fs.chmodSync(filePath, 0);
        }
      });
    });

    // A truncated archive looks like a report to whoever is sent it, and
    // doctor cannot analyse it
    it('should not leave a partial archive behind', async () => {
      await expect(archiveSamples(samples, outputDir)).to.be.rejectedWith('EACCES');

      expect(fs.readdirSync(outputDir)).to.be.empty();
    });

    // A report already at that path was not written by this run
    it('should keep a report that was already at the destination', async () => {
      const existingReport = path.join(outputDir, 'dashmate-report-2026-11-26T10-15-30.123Z.tar.gz');
      fs.writeFileSync(existingReport, 'previous report');

      await expect(archiveSamples(samples, outputDir)).to.be.rejectedWith('EACCES');

      expect(fs.readFileSync(existingReport, 'utf8')).to.equal('previous report');
    });

    // The packing failure is what the operator has to act on
    it('should report the packing error when the cleanup fails too', async function it() {
      const { rmSync } = fs;
      this.sinon.stub(fs, 'rmSync').callsFake((filePath, ...args) => {
        if (filePath.startsWith(outputDir)) {
          throw Object.assign(new Error('EPERM: operation not permitted'), { code: 'EPERM' });
        }

        return rmSync(filePath, ...args);
      });

      await expect(archiveSamples(samples, outputDir)).to.be.rejectedWith('EACCES');
    });
  });
});
