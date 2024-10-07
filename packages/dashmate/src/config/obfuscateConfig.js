import lodash from 'lodash';
import obfuscateObjectRecursive from '../util/obfuscateObjectRecursive.js';
import Config from './Config.js';
import hideString from '../util/hideString.js';

/**
 * @param {Config} config
 * @return {Config}
 */
export default function obfuscateConfig(
  config,
) {
  const username = process.env.USER;

  const clonedOptions = lodash.cloneDeep(config.getOptions());

  // sanitize [password, apiKey, privateKey, externalIp] fields in the dashmate config
  obfuscateObjectRecursive(clonedOptions, (field, value) => (typeof value === 'string' && field === 'password' ? hideString(value) : value));
  obfuscateObjectRecursive(clonedOptions, (field, value) => (typeof value === 'string' && field === 'key' ? hideString(value) : value));
  obfuscateObjectRecursive(clonedOptions, (field, value) => (typeof value === 'string' && field === 'apiKey' ? hideString(value) : value));
  obfuscateObjectRecursive(clonedOptions, (field, value) => (typeof value === 'string' && field === 'privateKey' ? hideString(value) : value));

  // sanitize also usernames & external ip from the rest of the fields values
  obfuscateObjectRecursive(clonedOptions, (_field, value) => (typeof value === 'string' ? value.replaceAll(
    username,
    hideString(username),
  ) : value));

  return new Config(config.getName(), clonedOptions);
}
