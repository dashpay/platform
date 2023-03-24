const fs = require('fs');

function validateFileExists(value) {
  // TODO: We should make sure that file is accessable
  // TODO: We should make sure it's a file not a directory
  if (fs.existsSync(value)) {
    return true;
  }

  return 'File doesn\'t exist';
}

module.exports = validateFileExists;
