import fs from 'fs';
import Ajv from 'ajv';
import path from 'path';
import { Config } from '../Config.js';
import { PACKAGE_ROOT_DIR } from '../../constants.js';
import { ConfigFileNotFoundError } from '../errors/ConfigFileNotFoundError.js';
import { InvalidConfigFileFormatError } from '../errors/InvalidConfigFileFormatError.js';
import configFileJsonSchema from './configFileJsonSchema.js';
import { ConfigFile } from './ConfigFile.js';

export class ConfigFileJsonRepository {
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
   * @returns {ConfigFile}
   */
  read() {
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

    const { version } = JSON.parse(fs.readFileSync(path.join(PACKAGE_ROOT_DIR, 'package.json'), 'utf8'));

    const originConfigVersion = configFileData.configFormatVersion;

    const migratedConfigFileData = this.migrateConfigFile(
      configFileData,
      configFileData.configFormatVersion,
      version,
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
