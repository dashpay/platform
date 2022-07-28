const fs = require('fs');
const path = require('path');

/**
 * @param {string} homeDirPath
 * @return {checkCertificateTask}
 */
function checkCertificateTaskFactory(homeDirPath) {
  /**
   * @typedef {checkCertificateTask}
   * @param {string} fileName
   * @return {boolean}
   */
  function checkCertificateTask(fileName) {
    const filePath = path.resolve(homeDirPath, 'ssl', fileName);
    let certStatus;
    try {
      if (fs.existsSync(filePath)) {
        certStatus = true;
      } else {
        certStatus = false;
      }
    } catch (e) {
      throw new Error(e);
    }
    return certStatus;
  }

  return checkCertificateTask;
}

module.exports = checkCertificateTaskFactory;
