const fs = require('fs');

const Ajv = require('ajv');

const Config = require('../Config');
const ConfigFile = require('./ConfigFile');

const configFileJsonSchema = require('../../../configs/schema/configFileJsonSchema');

const ConfigFileNotFoundError = require('../errors/ConfigFileNotFoundError');
const InvalidConfigFileFormatError = require('../errors/InvalidConfigFileFormatError');

const packageJson = require('../../../package.json');

class ConfigFileJsonRepository {
  /**
   * @param {string} configFilePath
   * @param {migrateConfigFile} migrateConfigFile
   */
  constructor(configFilePath, migrateConfigFile) {
    this.configFilePath = configFilePath;
    this.migrateConfigFile = migrateConfigFile;
    this.ajv = new Ajv();
  }

  /**
   * Load configs from file
   *
   * @returns {Promise<ConfigFile>}
   */
  async read() {
    if (!fs.existsSync(this.configFilePath)) {
      throw new ConfigFileNotFoundError(this.configFilePath);
    }

    const configFileJSON = fs.readFileSync(this.configFilePath, 'utf8');

    let configFileData;
    try {
      configFileData = JSON.parse(configFileJSON);
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }

    const migratedConfigFileData = this.migrateConfigFile(
      configFileData,
      configFileData.configFormatVersion,
      packageJson.version,
    );

    const isValid = this.ajv.validate(configFileJsonSchema, migratedConfigFileData);

    if (!isValid) {
      const error = new Error(this.ajv.errorsText(undefined, { dataVar: 'configFile' }));

      throw new InvalidConfigFileFormatError(this.configFilePath, error);
    }

    let configs;
    try {
      configs = Object.entries(migratedConfigFileData.configs)
        .map(([name, options]) => new Config(name, options));
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }

    return new ConfigFile(
      configs,
      packageJson.version,
      migratedConfigFileData.defaultConfigName,
      migratedConfigFileData.defaultGroupName,
    );
  }

  /**
   * Save configs to file
   *
   * @param {ConfigFile} configFile
   * @returns {Promise<void>}
   */
  async write(configFile) {
    const configFileJSON = JSON.stringify(configFile.toObject(), undefined, 2);

    fs.writeFileSync(this.configFilePath, configFileJSON, 'utf8');
  }
}

module.exports = ConfigFileJsonRepository;
