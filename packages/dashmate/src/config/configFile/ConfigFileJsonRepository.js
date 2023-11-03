const fs = require('fs');

const Ajv = require('ajv');

const Config = require('../Config');

const ConfigFile = require('./ConfigFile');

const configFileJsonSchema = require('./configFileJsonSchema');
const ConfigFileNotFoundError = require('../errors/ConfigFileNotFoundError');

const InvalidConfigFileFormatError = require('../errors/InvalidConfigFileFormatError');
const packageJson = require('../../../package.json');

class ConfigFileJsonRepository {
  /**
   * @param {migrateConfigFile} migrateConfigFile
   * @param {HomeDir} homeDir
   */
  constructor(migrateConfigFile, homeDir) {
    this.migrateConfigFile = migrateConfigFile;
    this.ajv = new Ajv();
    this.configFilePath = homeDir.joinPath('config.json');
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

    const originConfigVersion = configFileData.configFormatVersion;

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

    const configFile = new ConfigFile(
      configs,
      migratedConfigFileData.configFormatVersion,
      migratedConfigFileData.projectId,
      migratedConfigFileData.defaultConfigName,
      migratedConfigFileData.defaultGroupName,
    );

    // Mark configs as changed if they were migrated
    if (migratedConfigFileData.configFormatVersion !== originConfigVersion) {
      configFile.markAsChanged();
      configFile.getAllConfigs().forEach((config) => config.markAsChanged());
    }

    return configFile;
  }

  /**
   * Save configs to file
   *
   * @param {ConfigFile} configFile
   * @returns {void}
   */
  write(configFile) {
    const configFileJSON = JSON.stringify(configFile.toObject(), undefined, 2);

    configFile.markAsSaved();

    fs.writeFileSync(this.configFilePath, `${configFileJSON}\n`, 'utf8');
  }
}

module.exports = ConfigFileJsonRepository;
