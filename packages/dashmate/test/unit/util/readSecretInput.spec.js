import fs from 'fs';
import os from 'os';
import path from 'path';
import { Readable } from 'stream';
import { execFileSync } from 'child_process';
import readSecretInput, {
  MAX_SECRET_INPUT_BYTES,
  getReadableByOthersWarning,
} from '../../../src/util/readSecretInput.js';

describe('readSecretInput', () => {
  let dir;

  /**
   * @param {string} name
   * @param {string} content
   * @param {number} mode
   * @return {string}
   */
  function writeSecret(name, content, mode = 0o600) {
    const filePath = path.join(dir, name);
    fs.writeFileSync(filePath, content, { mode });
    fs.chmodSync(filePath, mode);

    return filePath;
  }

  /**
   * @param {string} content
   * @return {Readable}
   */
  function pipedStdin(content) {
    const stream = Readable.from([Buffer.from(content)]);
    stream.isTTY = undefined;

    return stream;
  }

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dashmate-secret-'));
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it('should read a protected file without a warning', async () => {
    const filePath = writeSecret('key', 'secret\n');

    const result = await readSecretInput(filePath, { flag: '--platform-node-key-file' });

    expect(result).to.deep.equal({ content: 'secret\n', warnings: [] });
  });

  // The input is a key: anyone else who can read the file can impersonate the
  // node. Refusing would break operators whose umask is permissive, so it is a
  // warning that names the fix.
  it('should warn when other users can read the file', async function it() {
    if (process.platform === 'win32') {
      this.skip();
    }

    const filePath = writeSecret('key', 'secret', 0o644);

    const { warnings } = await readSecretInput(filePath, { flag: '--platform-node-key-file' });

    expect(warnings).to.have.lengthOf(1);
    expect(warnings[0]).to.include(filePath).and.include('chmod 600');
  });

  it('should warn when the group can read the file', async function it() {
    if (process.platform === 'win32') {
      this.skip();
    }

    const filePath = writeSecret('key', 'secret', 0o640);

    const { warnings } = await readSecretInput(filePath, { flag: '--platform-node-key-file' });

    expect(warnings).to.have.lengthOf(1);
  });

  it('should read stdin when the path is -', async () => {
    const result = await readSecretInput('-', {
      flag: '--platform-node-key-file',
      stdin: pipedStdin('from stdin\n'),
    });

    expect(result).to.deep.equal({ content: 'from stdin\n', warnings: [] });
  });

  // Reading a terminal would wait for the key to be typed, and echo it.
  it('should refuse - when stdin is a terminal', async () => {
    const stdin = pipedStdin('');
    stdin.isTTY = true;

    await expect(readSecretInput('-', { flag: '--platform-node-key-file', stdin }))
      .to.be.rejectedWith(/terminal/);
  });

  it('should refuse a file larger than the limit without reading it into a message', async () => {
    const filePath = writeSecret('key', 'x'.repeat(MAX_SECRET_INPUT_BYTES + 1));

    const error = await readSecretInput(filePath, { flag: '--platform-node-key-file' })
      .then(() => null, (e) => e);

    expect(error.message).to.include(`${MAX_SECRET_INPUT_BYTES} bytes`);
    expect(error.message).to.not.include('xxxxxxxx');
  });

  it('should refuse stdin larger than the limit', async () => {
    await expect(readSecretInput('-', {
      flag: '--platform-node-key-file',
      stdin: pipedStdin('x'.repeat(MAX_SECRET_INPUT_BYTES + 1)),
    })).to.be.rejectedWith(`${MAX_SECRET_INPUT_BYTES} bytes`);
  });

  it('should refuse a directory', async () => {
    await expect(readSecretInput(dir, { flag: '--platform-node-key-file' }))
      .to.be.rejectedWith(/regular file/);
  });

  // Opening a FIFO for reading would wait for a writer, and with Ctrl-C
  // handled by the command only kill -9 would end it.
  it('should refuse a FIFO at once instead of waiting for a writer', async function it() {
    if (process.platform === 'win32') {
      this.skip();
    }

    const fifo = path.join(dir, 'fifo');
    execFileSync('mkfifo', [fifo]);

    await expect(readSecretInput(fifo, { flag: '--platform-node-key-file' }))
      .to.be.rejectedWith(/regular file/);
  });

  // A TLS private key can be larger than the limit for node keys.
  it('should check the mode of a large file without reading it', function it() {
    if (process.platform === 'win32') {
      this.skip();
    }

    const filePath = writeSecret('tls.key', 'x'.repeat(MAX_SECRET_INPUT_BYTES * 3), 0o644);

    expect(getReadableByOthersWarning(filePath, '--ssl-private-key-file')).to.include('chmod 600');
    fs.chmodSync(filePath, 0o600);
    expect(getReadableByOthersWarning(filePath, '--ssl-private-key-file')).to.be.null();
  });

  it('should name the flag and the path when the file is missing', async () => {
    const missing = path.join(dir, 'missing');

    await expect(readSecretInput(missing, { flag: '--platform-node-key-file' }))
      .to.be.rejectedWith(`--platform-node-key-file: cannot read '${missing}': ENOENT`);
  });
});
