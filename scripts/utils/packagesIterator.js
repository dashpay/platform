const path = require('path');
const { workspaces } = require('../../package.json');

/**
 *
 * @param {string} packagesDir
 * @return {Generator<{filename: string, json: Object}, void, *>}
 */
module.exports = function *packagesIterator() {
  const rootDir = path.join(__dirname, '..', '..');

  for (const item of workspaces) {
    const packageFile = path.join(rootDir , item, 'package.json');

    yield { filename: packageFile, json: require(packageFile) };
  }
};

