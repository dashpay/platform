const fs = require('fs');

const Ajv = require('ajv');

const Config = require('../Config');
const ConfigCollection = require('../ConfigCollection');

const configFileJsonSchema = require('./configFileJsonSchema');

const ConfigFileNotFoundError = require('../errors/ConfigFileNotFoundError');
const InvalidConfigFileFormatError = require('../errors/InvalidConfigFileFormatError');

const packageJson = require('../../../package.json');

class ConfigJsonFileRepository {
  /**
   * @param configFilePath
   */
  constructor(configFilePath, migrateConfigOptions) {
    this.configFilePath = configFilePath;
    this.migrateConfigOptions = migrateConfigOptions;
    this.ajv = new Ajv();
  }

  /**
   * Load configs from file
   *
   * @returns {Promise<ConfigCollection>}
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

    const isValid = this.ajv.validate(configFileJsonSchema, configFileData);

    if (!isValid) {
      const error = new Error(this.ajv.errorsText(undefined, { dataVar: 'configFile' }));

      throw new InvalidConfigFileFormatError(this.configFilePath, error);
    }

    let configs;
    try {
      configs = Object.entries(configFileData.configs)
        .map(([name, options]) => {
          const migratedOptions = this.migrateConfigOptions(
            name,
            options,
            configFileData.configFormatVersion,
            packageJson.version,
          );

          return new Config(name, migratedOptions);
        });
    } catch (e) {
      throw new InvalidConfigFileFormatError(this.configFilePath, e);
    }

    return new ConfigCollection(configs, configFileData.defaultConfigName, packageJson.version);
  }

  /**
   * Save configs to file
   *
   * @param {ConfigCollection} configCollection
   * @returns {Promise<void>}
   */
  async write(configCollection) {
    const configFileData = {
      defaultConfigName: configCollection.getDefaultConfigName(),
      configFormatVersion: configCollection.getConfigFormatVersion(),
    };

    configFileData.configs = configCollection.getAllConfigs().reduce((configsMap, config) => {
      // eslint-disable-next-line no-param-reassign
      configsMap[config.getName()] = config.getOptions();

      return configsMap;
    }, {});

    const configFileJSON = JSON.stringify(configFileData, undefined, 2);

    fs.writeFileSync(this.configFilePath, configFileJSON, 'utf8');
  }
}

module.exports = ConfigJsonFileRepository;
