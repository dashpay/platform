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
    expect(certificateStorageWritable({ directories: [directory('ssl')] })).to.be.true();
  });

  // The node that has never held a certificate. It needs the request, not a
  // repair, and treating an absent directory as broken storage would withhold
  // the one command that would fix it.
  it('should not call a directory that does not exist yet a fault', () => {
    expect(certificateStorageWritable({ directories: [path.join(homeDir.getPath(), 'never-created')] }))
      .to.be.true();
  });

  it('should refuse a directory that cannot be written', function it() {
    // Running as root makes the mode advisory, and the probe would succeed.
    if (typeof process.getuid === 'function' && process.getuid() === 0) {
      this.skip();
    }

    const readOnly = directory('read-only', 0o500);

    try {
      expect(certificateStorageWritable({ directories: [readOnly] })).to.be.false();
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
      expect(certificateStorageWritable({ directories: [writable, readOnly] })).to.be.false();
    } finally {
      fs.chmodSync(readOnly, 0o700);
    }
  });

  // The gateway's own two files are overwritten in place rather than replaced,
  // so their permissions decide, not their directory's. A file owned by another
  // user sits in a directory that still happily accepts new files.
  it('should refuse a target file that cannot be overwritten', function it() {
    if (typeof process.getuid === 'function' && process.getuid() === 0) {
      this.skip();
    }

    const dir = directory('ssl');
    const bundle = path.join(dir, 'bundle.crt');

    fs.writeFileSync(bundle, 'certificate');
    fs.chmodSync(bundle, 0o400);

    try {
      expect(certificateStorageWritable({ directories: [dir], files: [bundle] })).to.be.false();
      // The directory alone would have cleared it.
      expect(certificateStorageWritable({ directories: [dir] })).to.be.true();
    } finally {
      fs.chmodSync(bundle, 0o600);
    }
  });

  it('should not fault a target file that is not there yet', () => {
    const dir = directory('ssl');

    expect(certificateStorageWritable({
      directories: [dir],
      files: [path.join(dir, 'bundle.crt')],
    })).to.be.true();
  });

  // Checked without truncating: this asks whether the certificate the gateway
  // is serving could be replaced, not whether it can be destroyed.
  it('should leave an existing target file untouched', () => {
    const dir = directory('ssl');
    const bundle = path.join(dir, 'bundle.crt');

    fs.writeFileSync(bundle, 'certificate');

    certificateStorageWritable({ directories: [dir], files: [bundle] });

    expect(fs.readFileSync(bundle, 'utf8')).to.equal('certificate');
  });

  // The probe is a real write of real size because the fault that matters
  // passes every permission check: a full disk is writable by mode, accepts an
  // empty inode, and refuses the bytes.
  it('should write actual content, not an empty file', () => {
    const dir = directory('ssl');
    let written = 0;

    const realWriteFileSync = fs.writeFileSync;
    fs.writeFileSync = (file, data, ...rest) => {
      written = data.length ?? 0;

      return realWriteFileSync(file, data, ...rest);
    };

    try {
      certificateStorageWritable({ directories: [dir] });
    } finally {
      fs.writeFileSync = realWriteFileSync;
    }

    expect(written).to.be.above(1024);
  });

  it('should leave nothing behind', () => {
    const dir = directory('ssl');

    certificateStorageWritable({ directories: [dir] });

    expect(fs.readdirSync(dir)).to.have.lengthOf(0);
  });
});
