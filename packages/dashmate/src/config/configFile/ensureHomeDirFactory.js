const fs = require('fs');

const CouldNotCreateHomeDirError = require('../errors/CouldNotCreateHomeDirError');
const HomeDirIsNotWritableError = require('../errors/HomeDirIsNotWritableError');

/**
 * @param {string} homeDirPath
 * @return {ensureHomeDir}
 */
function ensureHomeDirFactory(homeDirPath) {
  /**
   * @typedef {ensureHomeDir}
   * @return {string} homeDirPath
   */
  function ensureHomeDir() {
    if (fs.existsSync(homeDirPath)) {
      try {
        // eslint-disable-next-line no-bitwise
        fs.accessSync(__dirname, fs.constants.R_OK | fs.constants.W_OK);
      } catch (e) {
        throw new HomeDirIsNotWritableError(homeDirPath);
      }

      return homeDirPath;
    }

    try {
      fs.mkdirSync(homeDirPath);
    } catch (e) {
      throw new CouldNotCreateHomeDirError(homeDirPath);
    }

    return homeDirPath;
  }

  return ensureHomeDir;
}

module.exports = ensureHomeDirFactory;
