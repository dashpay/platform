import Ajv from 'ajv';
import lodash from 'lodash';

import addFormats from 'ajv-formats';
import configJsonSchema from './configJsonSchema.js';

import InvalidOptionPathError from './errors/InvalidOptionPathError.js';
import OptionIsNotSetError from './errors/OptionIsNotSetError.js';
import InvalidOptionError from './errors/InvalidOptionError.js';
import InvalidOptionsError from './errors/InvalidOptionsError.js';
import { assertSafeConfigName } from './resolve-config-directory.js';

const {
  get: lodashGet, set: lodashSet, cloneDeep: lodashCloneDeep, isEqual: lodashIsEqual,
} = lodash;

export default class Config {
  /**
   * @param {string} name
   * @param {Object} options
   * @param {boolean} [skipValidation=false] - Skip schema validation (use with --force)
   */
  constructor(name, options = {}, skipValidation = false) {
    assertSafeConfigName(name);

    this.name = name;
    this.changed = false;

    this.setOptions(options, skipValidation);
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
   * Check whether a path is reachable per the config JSON schema (regardless
   * of whether a value is currently set there).
   *
   * Use this when checking the legality of a `set` to a path that doesn't yet
   * have a value — notably under map-shaped properties whose schema uses
   * `additionalProperties: <schema>` (e.g. `…build.buildArgs.<KEY>`), where
   * `config.has(...)` will return `false` even though `config.set(...)` is
   * semantically legal.
   *
   * `configJsonSchema` IS the per-config schema — the top-level
   * `properties: { description, group, docker, core, platform, … }` describes
   * one config entry. Walks it descending through:
   * - `properties[segment]` (typed field),
   * - `additionalProperties` (variable-key map, only when the value is a
   *   schema object — `additionalProperties: false` blocks the descent),
   * - `$ref` references into `#/definitions/...`.
   *
   * @param {string} path - dot-separated option path (e.g.
   *   `'platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA'`).
   * @return {boolean} true when the path is allowed by the schema.
   */
  static isSchemaPathAllowed(path) {
    if (typeof path !== 'string' || path.length === 0) return false;

    // Reject empty segments (leading/trailing/double dots, e.g. `a..b` or
    // `…buildArgs.`) — an empty key must not slip through a map's
    // `additionalProperties` descent.
    const pathSegments = path.split('.');
    if (pathSegments.some((segment) => segment.length === 0)) return false;

    const resolveRef = (node) => {
      if (!node || typeof node !== 'object') return node;
      if (typeof node.$ref !== 'string') return node;
      const ref = node.$ref;
      if (!ref.startsWith('#/')) return null;
      const segments = ref.slice(2).split('/');
      let resolved = configJsonSchema;
      for (const seg of segments) {
        if (!resolved || typeof resolved !== 'object') return null;
        resolved = resolved[seg];
      }
      return resolveRef(resolved);
    };

    let node = resolveRef(configJsonSchema);
    if (!node) return false;

    for (const segment of pathSegments) {
      node = resolveRef(node);
      if (!node || typeof node !== 'object') return false;

      // Typed property.
      if (node.properties && Object.prototype.hasOwnProperty.call(node.properties, segment)) {
        node = node.properties[segment];
        continue;
      }

      // Map with a schema for extra keys — descend into the value schema.
      if (
        node.additionalProperties
        && typeof node.additionalProperties === 'object'
      ) {
        node = node.additionalProperties;
        continue;
      }

      // No match and no permissive additionalProperties — path not allowed.
      return false;
    }

    return true;
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
   * @param {boolean} [skipValidation=false] - Skip schema validation (use with --force)
   *
   * @return {Config}
   */
  setOptions(options, skipValidation = false) {
    const clonedOptions = lodashCloneDeep(options);

    if (!skipValidation) {
      const isValid = Config.ajv.validate(configJsonSchema, clonedOptions);

      if (!isValid) {
        const message = Config.ajv.errorsText(undefined, { dataVar: 'config' });

        throw new InvalidOptionsError(
          clonedOptions,
          Config.ajv.errors,
          message,
        );
      }
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
