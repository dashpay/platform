const path = require('path');
const TOML = require('@iarna/toml')
const {workspaces} = require('../../package.json');
const {readdirSync, readFileSync} = require('fs')

/**
 * @return {Generator<{filename: string, json: Object}, void, *>}
 */
module.exports = {
  npm: function *() {
    const rootDir = path.join(__dirname, '..', '..');

    for (const item of workspaces) {
      const packageFile = path.join(rootDir, item, 'package.json');

      yield {filename: packageFile, json: require(packageFile)};
    }
  },
  rust: function *() {
    const rootDir = path.join(__dirname, '..', '..');

    const packagesDir = path.join(rootDir, 'packages')

    const allPackages = readdirSync(packagesDir).filter(e => e !== 'README.md')

    const rustPackages = allPackages.filter(e => readdirSync(path.join(packagesDir, e)).indexOf('Cargo.toml') !== -1)

    for (const rustPackage of rustPackages) {
      const filename = path.join(packagesDir, rustPackage, 'Cargo.toml');
      const cargoFile = readFileSync(filename);

      yield {filename, toml: TOML.parse(cargoFile)};
    }
  },
};
