import lodash from 'lodash';
import obfuscateObjectRecursive from '../util/obfuscateObjectRecursive.js';
import Config from './Config.js';
import hideString from '../util/hideString.js';

/**
 * Config fields whose values are secret, at whatever depth they occur
 */
export const SECRET_FIELD_NAMES = ['password', 'key', 'apiKey', 'privateKey'];

/**
 * @param {Object} object
 * @param {Set<string>} values - collects the secret values found
 */
function collectSecretValues(object, values) {
  for (const [field, value] of Object.entries(object)) {
    if (typeof value === 'object' && value !== null) {
      collectSecretValues(value, values);
    } else if (typeof value === 'string' && SECRET_FIELD_NAMES.includes(field)) {
      values.add(value);
    }
  }
}

/**
 * The values of every secret field in a config, so they can be removed from
 * text the config itself is not part of
 *
 * @param {Config} config - as stored, not obfuscated
 * @return {string[]}
 */
export function getConfigSecretValues(config) {
  const values = new Set();

  collectSecretValues(config.getOptions(), values);

  return [...values];
}

/**
 * @param {Config} config
 * @return {Config}
 */
export default function obfuscateConfig(
  config,
) {
  const username = process.env.USER;

  const clonedOptions = lodash.cloneDeep(config.getOptions());

  obfuscateObjectRecursive(clonedOptions, (field, value) => (typeof value === 'string' && SECRET_FIELD_NAMES.includes(field) ? hideString(value) : value));

  // sanitize also usernames & external ip from the rest of the fields values
  obfuscateObjectRecursive(clonedOptions, (_field, value) => (typeof value === 'string' ? value.replaceAll(
    username,
    hideString(username),
  ) : value));

  return new Config(config.getName(), clonedOptions);
}
