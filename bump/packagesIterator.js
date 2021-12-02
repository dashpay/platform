import fs from 'fs';
import path from 'path';

module.exports = function *packagesIterator () {
  const packagesDir = path.join(__dirname, '..', 'packages');

  const items = fs.readdirSync(packagesDir);

  for (const item of items) {
    const fullPath = path.join(packagesDir, item);

    if (fs.lstatSync(fullPath).isDirectory()) {
      const packageFile = path.join(fullPath, 'package.json');

      yield { filename: packageFile, json: require(packageFile) };
    }
  }
};

