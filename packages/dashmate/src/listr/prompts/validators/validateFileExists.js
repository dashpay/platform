const fs = require('fs');

function validateFileExists(value) {
  if (fs.existsSync(value)) {
    return true;
  }

  return 'File doesn\'t exist';
}

module.exports = validateFileExists;
