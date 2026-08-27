import fs from 'fs';
import path from 'path';
import certificateStorageWritable from '../../../src/ssl/certificateStorageWritable.js';
import HomeDir from '../../../src/config/HomeDir.js';

describe('certificateStorageWritable', () => {
  let homeDir;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
  });

  afterEach(() => homeDir.remove());

  /**
   * @param {string} name
   * @param {number} [mode]
   * @return {string}
   */
  function directory(name, mode) {
    const created = path.join(homeDir.getPath(), name);

    fs.mkdirSync(created, { recursive: true });

    if (mode !== undefined) {
      fs.chmodSync(created, mode);
    }

    return created;
  }

  it('should accept a directory it can write to', () => {
    expect(certificateStorageWritable([directory('ssl')])).to.be.true();
  });

  // The node that has never held a certificate. It needs the request, not a
  // repair, and treating an absent directory as broken storage would withhold
  // the one command that would fix it.
  it('should not call a directory that does not exist yet a fault', () => {
    expect(certificateStorageWritable([path.join(homeDir.getPath(), 'never-created')]))
      .to.be.true();
  });

  it('should refuse a directory that cannot be written', function it() {
    // Running as root makes the mode advisory, and the probe would succeed.
    if (typeof process.getuid === 'function' && process.getuid() === 0) {
      this.skip();
    }

    const readOnly = directory('read-only', 0o500);

    try {
      expect(certificateStorageWritable([readOnly])).to.be.false();
    } finally {
      fs.chmodSync(readOnly, 0o700);
    }
  });

  it('should refuse when any one of the directories refuses', function it() {
    if (typeof process.getuid === 'function' && process.getuid() === 0) {
      this.skip();
    }

    const writable = directory('lego');
    const readOnly = directory('ssl-read-only', 0o500);

    try {
      expect(certificateStorageWritable([writable, readOnly])).to.be.false();
    } finally {
      fs.chmodSync(readOnly, 0o700);
    }
  });

  // The probe is a real write because the fault that matters passes every
  // permission check: a full disk is writable by mode and refuses the write.
  it('should leave nothing behind', () => {
    const dir = directory('ssl');

    certificateStorageWritable([dir]);

    expect(fs.readdirSync(dir)).to.have.lengthOf(0);
  });
});
