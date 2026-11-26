import fs from 'fs';

/**
 * Read a file once, through a descriptor checked to be a regular file's
 *
 * Opening without blocking keeps a FIFO from waiting for a writer, and the
 * check is on what was opened, so the path cannot be swapped in between.
 *
 * @param {string} filePath
 * @return {{ content: Buffer, stats: fs.Stats }|null} null when it is not a
 *  regular file
 */
export default function readRegularFile(filePath) {
  // eslint-disable-next-line no-bitwise
  const fd = fs.openSync(filePath, fs.constants.O_RDONLY | fs.constants.O_NONBLOCK);

  try {
    const stats = fs.fstatSync(fd);

    if (!stats.isFile()) {
      return null;
    }

    return { content: fs.readFileSync(fd), stats };
  } finally {
    fs.closeSync(fd);
  }
}
