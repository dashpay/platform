const Ajv = require('ajv');

const lodashGet = require('lodash/get');
const lodashSet = require('lodash/set');
const lodashCloneDeep = require('lodash/cloneDeep');
const lodashIsEqual = require('lodash/isEqual');

const addFormats = require('ajv-formats');
const configJsonSchema = require('./configJsonSchema');

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
    this.changed = false;

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
      const [error] = Config.ajv.errors;

      const pathSegments = path.split('.');
      pathSegments.pop();
      const parentPath = `/${pathSegments.join('/')}`;

      if (error.keyword === 'additionalProperties' && error.instancePath === parentPath) {
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

    this.changed = true;

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

    this.changed = true;

    return this;
  }

  /**
   * Compare two configs
   *
   * @param {Config} config
   * @returns {boolean}
   */
  isEqual(config) {
    return lodashIsEqual(this.getOptions(), config.getOptions());
  }

  /**
   * Is config changed
   *
   * @return {boolean}
   */
  isChanged() {
    return this.changed;
  }

  /**
   * Mark config as changed
   */
  markAsChanged() {
    this.changed = true;
  }

  /**
   * Mark config as saved
   */
  markAsSaved() {
    this.changed = false;
  }
}

Config.ajv = new Ajv({ coerceTypes: true });
addFormats(Config.ajv, { mode: 'fast', formats: ['ipv4'] });

module.exports = Config;
