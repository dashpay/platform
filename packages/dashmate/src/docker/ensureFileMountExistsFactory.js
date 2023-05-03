const fs = require('fs');
const path = require('path');

/**
 * @return {resolveDockerHostIp}
 */
function ensureFileMountExistsFactory() {
  /**
   * @typedef {resolveDockerHostIp}
   * @return {Promise<string>}
   */
  function ensureFileMountExists(logFilePath) {
    // Remove directory that could potentially be created by Docker mount
    if (fs.existsSync(logFilePath) && fs.lstatSync(logFilePath).isDirectory()) {
      fs.rmSync(logFilePath, { recursive: true });
    }

    if (!fs.existsSync(logFilePath)) {
      fs.mkdirSync(path.dirname(logFilePath), { recursive: true });
      fs.writeFileSync(logFilePath, '');
    }
  }

  return ensureFileMountExists;
}

module.exports = ensureFileMountExistsFactory;
