const Ajv = require('ajv');

const path = require('path');

const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');
const lodashCloneDeep = require('lodash.clonedeep');

const addFormats = require('ajv-formats');
const configJsonSchema = require('../../configs/schema/configJsonSchema');

const convertObjectToEnvs = require('./convertObjectToEnvs');

const InvalidOptionPathError = require('./errors/InvalidOptionPathError');
const InvalidOptionError = require('./errors/InvalidOptionError');
const InvalidOptionsError = require('./errors/InvalidOptionsError');
const OptionIsNotSetError = require('./errors/OptionIsNotSetError');

class Config {
  /**
   * @param {string} name
   * @param {Object} options
   */
  constructor(name, options = {}) {
    this.name = name;

    this.setOptions(options);
  }

  /**
   * Get name
   *
   * @return {string}
   */
  getName() {
    return this.name;
  }

  /**
   * Is option present
   *
   * @param {string} path
   * @return {boolean}
   */
  has(path) {
    return lodashGet(this.options, path) !== undefined;
  }

  /**
   * Get config option
   *
   * @param {string} path
   * @param {boolean} [isRequired=false]
   *
   * @return {*}
   */
  get(path, isRequired = false) {
    const value = lodashGet(this.options, path);

    if (value === undefined) {
      throw new InvalidOptionPathError(path);
    }

    if (isRequired && value === null) {
      throw new OptionIsNotSetError(this, path);
    }

    return value;
  }

  /**
   * Set config option
   *
   * @param {string} path
   * @param {*} value
   *
   * @return {Config}
   */
  set(path, value) {
    const clonedOptions = lodashCloneDeep(this.options);

    lodashSet(clonedOptions, path, lodashCloneDeep(value));

    const isValid = Config.ajv.validate(configJsonSchema, clonedOptions);

    if (!isValid) {
      if (Config.ajv.errors[0].keyword === 'additionalProperties') {
        throw new InvalidOptionPathError(path);
      }

      const message = Config.ajv.errorsText(undefined, { dataVar: 'config' });

      throw new InvalidOptionError(
        path,
        value,
        Config.ajv.errors,
        message,
      );
    }

    this.options = clonedOptions;

    return this;
  }

  /**
   * Get options
   *
   * @return {Object}
   */
  getOptions() {
    return this.options;
  }

  /**
   * Set options
   *
   * @param {Object} options
   *
   * @return {Config}
   */
  setOptions(options) {
    const clonedOptions = lodashCloneDeep(options);

    const isValid = Config.ajv.validate(configJsonSchema, clonedOptions);

    if (!isValid) {
      const message = Config.ajv.errorsText(undefined, { dataVar: 'config' });

      throw new InvalidOptionsError(
        clonedOptions,
        Config.ajv.errors,
        message,
      );
    }

    this.options = clonedOptions;

    return this;
  }

  /**
   *
   * @return {{CONFIG_NAME: string, COMPOSE_PROJECT_NAME: string}}
   */
  toEnvs() {
    const dockerComposeFiles = ['docker-compose.yml'];

    if (this.get('core.masternode.enable') === true) {
      dockerComposeFiles.push('docker-compose.sentinel.yml');
    }

    if (this.has('platform')) {
      dockerComposeFiles.push('docker-compose.platform.yml');

      if (this.get('platform.sourcePath') !== null) {
        dockerComposeFiles.push('docker-compose.platform.build.yml');
      }
    }

    let logDirectoryPath = '/dev/null';
    if (this.has('platform.drive.abci.log.prettyFile.path')) {
      logDirectoryPath = path.dirname(
        this.get('platform.drive.abci.log.prettyFile.path'),
      );
    }

    let logPrettyFileName = 'drive-pretty.log';
    if (this.has('platform.drive.abci.log.prettyFile.path')) {
      logPrettyFileName = path.basename(
        this.get('platform.drive.abci.log.prettyFile.path'),
      );
    }

    let logJsonFileName = 'drive-json.log';
    if (this.has('platform.drive.abci.log.jsonFile.path')) {
      logJsonFileName = path.basename(
        this.get('platform.drive.abci.log.jsonFile.path'),
      );
    }

    return {
      CONFIG_NAME: this.getName(),
      COMPOSE_PROJECT_NAME: `dash_masternode_${this.getName()}`,
      COMPOSE_FILE: dockerComposeFiles.join(':'),
      COMPOSE_PATH_SEPARATOR: ':',
      COMPOSE_DOCKER_CLI_BUILD: 1,
      DOCKER_BUILDKIT: 1,
      PLATFORM_DRIVE_ABCI_LOG_DIRECTORY_PATH: logDirectoryPath,
      PLATFORM_DRIVE_ABCI_LOG_PRETTY_FILE_NAME: logPrettyFileName,
      PLATFORM_DRIVE_ABCI_LOG_JSON_FILE_NAME: logJsonFileName,
      ...convertObjectToEnvs(this.getOptions()),
    };
  }
}

Config.ajv = new Ajv({ coerceTypes: true });
addFormats(Config.ajv, { mode: 'fast', formats: ['ipv4'] });

module.exports = Config;
