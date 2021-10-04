const fs = require('fs');

const CouldNotCreateHomeDirError = require('./config/errors/CouldNotCreateHomeDirError');
const HomeDirIsNotWritableError = require('./config/errors/HomeDirIsNotWritableError');

const { HOME_DIR_PATH } = require('./constants');

/**
 * @return {ensureHomeDir}
 */
function ensureHomeDirFactory() {
  /**
   * @typedef {ensureHomeDir}
   * @return {string} homeDirPath
   */
  function ensureHomeDir() {
    if (fs.existsSync(HOME_DIR_PATH)) {
      try {
        // eslint-disable-next-line no-bitwise
        fs.accessSync(__dirname, fs.constants.R_OK | fs.constants.W_OK);
      } catch (e) {
        throw new HomeDirIsNotWritableError(HOME_DIR_PATH);
      }

      return HOME_DIR_PATH;
    }

    try {
      fs.mkdirSync(HOME_DIR_PATH);
    } catch (e) {
      throw new CouldNotCreateHomeDirError(HOME_DIR_PATH);
    }

    return HOME_DIR_PATH;
  }

  return ensureHomeDir;
}

module.exports = ensureHomeDirFactory;
