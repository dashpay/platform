import fs from 'fs';

/**
 * Far more than any key or API key dashmate reads, and small enough that a
 * wrong path (a log, a disk image) fails at once instead of being read whole.
 */
export const MAX_SECRET_INPUT_BYTES = 4096;

/**
 * A warning when users other than the owner can read a file holding a secret
 *
 * Windows keeps file access in ACLs, which mode bits do not describe.
 *
 * @param {string} filePath
 * @param {fs.Stats} stats
 * @param {string} flag
 * @return {string|null}
 */
function describeLooseMode(filePath, stats, flag) {
  // eslint-disable-next-line no-bitwise
  if (process.platform === 'win32' || (stats.mode & 0o044) === 0) {
    return null;
  }

  return `${flag}: other users of this machine can read '${filePath}'.`
    + ` Restrict it with: chmod 600 '${filePath}'`;
}

/**
 * The same warning for a file that is read elsewhere, at any size
 *
 * @param {string} filePath
 * @param {string} flag
 * @param {fs.Stats} [stats] - of the file as it was read, when the caller has
 *  them
 * @return {string|null}
 */
export function getReadableByOthersWarning(filePath, flag, stats = undefined) {
  return describeLooseMode(filePath, stats ?? fs.statSync(filePath), flag);
}

/**
 * Read a secret an operator hands to a command through a file or stdin
 *
 * Secrets never come through argv, which other users see in `ps` and the shell
 * keeps in its history. Messages name the flag and the path, never what was
 * read.
 *
 * @typedef {function} readSecretInput
 * @param {string} source - a file path, or '-' for stdin
 * @param {Object} options
 * @param {string} options.flag - the flag the source was given with
 * @param {Readable} [options.stdin]
 * @return {Promise<{ content: string, warnings: string[] }>}
 */
export default async function readSecretInput(source, { flag, stdin = process.stdin }) {
  const tooLarge = () => new Error(`${flag}: the input is larger than ${MAX_SECRET_INPUT_BYTES} bytes;`
    + ' it should hold one key');

  if (source === '-') {
    // Reading a terminal waits for the key to be typed and echoes it.
    if (stdin.isTTY === true) {
      throw new Error(`${flag}: stdin is a terminal. Pipe the key in, or give a file path`);
    }

    const chunks = [];
    let size = 0;
    for await (const chunk of stdin) {
      size += chunk.length;
      if (size > MAX_SECRET_INPUT_BYTES) {
        throw tooLarge();
      }
      chunks.push(Buffer.from(chunk));
    }

    return { content: Buffer.concat(chunks).toString('utf8'), warnings: [] };
  }

  let fd;
  try {
    // Non-blocking, so a FIFO does not wait for a writer that never comes:
    // it opens at once and is then refused as not a regular file.
    // eslint-disable-next-line no-bitwise
    fd = fs.openSync(source, fs.constants.O_RDONLY | fs.constants.O_NONBLOCK);
  } catch (e) {
    throw new Error(`${flag}: cannot read '${source}': ${e.code ?? e.message}`);
  }

  try {
    const stats = fs.fstatSync(fd);

    if (!stats.isFile()) {
      throw new Error(`${flag}: '${source}' is not a regular file`);
    }

    if (stats.size > MAX_SECRET_INPUT_BYTES) {
      throw tooLarge();
    }

    // Read no more than the limit allows, in case the file grew since.
    const buffer = Buffer.alloc(MAX_SECRET_INPUT_BYTES + 1);
    const length = fs.readSync(fd, buffer, 0, buffer.length, 0);

    if (length > MAX_SECRET_INPUT_BYTES) {
      throw tooLarge();
    }

    const warning = describeLooseMode(source, stats, flag);

    return {
      content: buffer.subarray(0, length).toString('utf8'),
      warnings: warning ? [warning] : [],
    };
  } finally {
    fs.closeSync(fd);
  }
}
