import fs from 'fs';
import path from 'path';

/**
 * @return {ensureFileMountExists}
 */

export function ensureFileMountExistsFactory() {
  /**
   * @typedef {ensureFileMountExists}
   * @param {string} filePath
   * @param {string|number} [mode] - https://nodejs.org/api/fs.html#fschmodpath-mode-callback
   * @return {void}
   */
  function ensureFileMountExists(filePath, mode = undefined) {
    // Remove directory that could potentially be created by Docker mount
    if (fs.existsSync(filePath) && fs.lstatSync(filePath).isDirectory()) {
      fs.rmSync(filePath, { recursive: true });
    }

    if (!fs.existsSync(filePath)) {
      fs.mkdirSync(path.dirname(filePath), { recursive: true });
      fs.writeFileSync(filePath, '');
    }

    // applies permission on each run
    if (mode !== undefined) {
      fs.chmodSync(filePath, mode);
    }
  }

  return ensureFileMountExists;
}
