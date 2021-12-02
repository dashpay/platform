const fs = require('fs');
const path = require('path');

/**
 *
 * @param {string} packagesDir
 * @return {Generator<{filename: string, json: Object}, void, *>}
 */
module.exports = function *packagesIterator (packagesDir) {
  const items = fs.readdirSync(packagesDir);

  for (const item of items) {
    const fullPath = path.join(packagesDir, item);

    if (fs.lstatSync(fullPath).isDirectory()) {
      const packageFile = path.join(fullPath, 'package.json');

      yield { filename: packageFile, json: require(packageFile) };
    }
  }
};

