import Ajv from 'ajv';
import lodash from 'lodash';

import addFormats from 'ajv-formats';
import configJsonSchema from './configJsonSchema.js';

import InvalidOptionPathError from './errors/InvalidOptionPathError.js';
import OptionIsNotSetError from './errors/OptionIsNotSetError.js';
import InvalidOptionError from './errors/InvalidOptionError.js';
import InvalidOptionsError from './errors/InvalidOptionsError.js';
import { assertSafeConfigName } from './resolve-config-directory.js';
import { resolveDerivedDefaults } from './derivedDefaults.js';

const {
  get: lodashGet, set: lodashSet, cloneDeep: lodashCloneDeep, isEqual: lodashIsEqual,
} = lodash;

function deepFreeze(value) {
  if (value === null || typeof value !== 'object' || Object.isFrozen(value)) {
    return value;
  }

  Object.values(value).forEach(deepFreeze);

  return Object.freeze(value);
}

export default class Config {
  #storedOptions = {};

  #effectiveOptions = {};

  /**
   * @param {string} name
   * @param {Object} options
   * @param {boolean} [skipValidation=false] - Skip schema validation (use with --force)
   */
  constructor(name, options = {}, skipValidation = false) {
    assertSafeConfigName(name);

    this.name = name;

    this.setOptions(options, skipValidation);

    // Hydration is not a mutation. setOptions() marks the config changed because
    // it is a genuine edit when called on an existing config, but a config that
    // was just loaded has nothing unsaved. Callers that build a config which must
    // reach disk - the default set for a new config file, createConfig() - mark it
    // changed themselves.
    this.changed = false;
  }

  /**
   * Options with version-derived defaults filled in.
   *
   * Kept as a property for callers that render or spread the whole config.
   *
   * @return {Object}
   */
  get options() {
    return this.#effectiveOptions;
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
    return lodashGet(this.#effectiveOptions, path) !== undefined;
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
    // Detached and frozen: detached so a caller assigning this into another
    // config cannot alias the two together, frozen so writing through it fails
    // loudly instead of being silently discarded. Callers that need to build new
    // state from a default read it with getStored(), which returns a mutable
    // copy of what is actually recorded.
    const value = deepFreeze(lodashCloneDeep(lodashGet(this.#effectiveOptions, path)));

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
    const clonedOptions = lodashCloneDeep(this.#storedOptions);

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

    this.#store(clonedOptions);

    this.changed = true;

    return this;
  }

  /**
   * Get options, with version-derived defaults filled in.
   *
   * This is the ordinary read: callers that render, display or serialize a
   * config get effective values without having to know a default exists. Stored
   * intent is reachable through getStoredOptions(), whose name makes the choice
   * deliberate.
   *
   * @return {Object}
   */
  getOptions() {
    return this.#effectiveOptions;
  }

  /**
   * Get an option exactly as stored, without derived defaults.
   *
   * Only persistence, cloning, intent equality, base-to-network inheritance and
   * reset may use this - everything else must read effective values, or an
   * unset option leaks out as null.
   *
   * @param {string} path
   * @return {*}
   */
  getStored(path) {
    const value = lodashGet(this.#storedOptions, path);

    if (value === undefined) {
      throw new InvalidOptionPathError(path);
    }

    return lodashCloneDeep(value);
  }

  /**
   * Get options exactly as stored, without derived defaults.
   *
   * @return {Object}
   */
  getStoredOptions() {
    return lodashCloneDeep(this.#storedOptions);
  }

  /**
   * Serialize to effective values, so JSON.stringify(config) matches what the
   * node actually runs.
   *
   * Whole-config reads are frozen: they back rendering and serialization, so a
   * write through one is always a mistake. Single values from get() are cloned
   * instead, because callers legitimately build new state from them.
   *
   * @return {Object}
   */
  toJSON() {
    // Shape matters: doctor archives serialize a config here and rebuild it with
    // new Config(name, options) on the way back in.
    return {
      name: this.name,
      options: this.#effectiveOptions,
    };
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

    this.#store(clonedOptions);

    this.changed = true;

    return this;
  }

  /**
   * Replace stored state and rebuild the effective snapshot from it.
   *
   * The snapshot is frozen: code that mutates the object returned by get() would
   * otherwise write into a copy and lose the change silently.
   *
   * @param {Object} options
   */
  #store(options) {
    this.#storedOptions = options;
    this.#effectiveOptions = deepFreeze(resolveDerivedDefaults(options));
  }

  /**
   * Compare two configs
   *
   * @param {Config} config
   * @returns {boolean}
   */
  isEqual(config) {
    return lodashIsEqual(this.getStoredOptions(), config.getStoredOptions());
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
addFormats(Config.ajv, { mode: 'fast', formats: ['ipv4', 'uri'] });
