const os = require('os');
const fs = require('fs');

function isWSL() {
  if (process.platform !== 'linux') {
    return false;
  }

  if (os.release().toLowerCase().includes('microsoft')) {
    return true;
  }

  try {
    return fs.readFileSync('/proc/version', 'utf8').toLowerCase().includes('microsoft');
  } catch (_) {
    return false;
  }
}

module.exports = isWSL;
