const fs = require('fs');
const path = require('path');

/**
 * @return {resolveDockerHostIp}
 */
function ensureFileMountExistsFactory() {
  /**
   * @typedef {resolveDockerHostIp}
   * @param logFilePath {string}
   * @param mode {mode} https://nodejs.org/api/fs.html#fschmodpath-mode-callback
   * @return {Promise<string>}
   */
  function ensureFileMountExists(logFilePath, mode) {
    // Remove directory that could potentially be created by Docker mount
    if (fs.existsSync(logFilePath) && fs.lstatSync(logFilePath).isDirectory()) {
      fs.rmSync(logFilePath, { recursive: true });
    }

    if (!fs.existsSync(logFilePath)) {
      fs.mkdirSync(path.dirname(logFilePath), { recursive: true });
      fs.writeFileSync(logFilePath, '');
    }

    // applies permission on each run
    if (mode) {
      fs.chmodSync(logFilePath, mode);
    }
  }

  return ensureFileMountExists;
}

module.exports = ensureFileMountExistsFactory;
