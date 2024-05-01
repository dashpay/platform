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

    const rootCargoPath = path.join(rootDir, 'Cargo.toml');
    const rootCargoFileString = readFileSync(rootCargoPath);

    const { workspace : { members }} = TOML.parse(rootCargoFileString);

    for (const rustPackage of members) {
      const filename = path.join(rootDir, rustPackage, 'Cargo.toml');
      const cargoFile = readFileSync(filename);

      yield {filename, toml: TOML.parse(cargoFile)};
    }
  },
};
