import lodash from 'lodash';
import obfuscateObjectRecursive from '../util/obfuscateObjectRecursive.js';
import hideString from '../util/hideString.js';

export default function obfuscateConfig(
  config,
) {
  const username = process.env.USER;
  const externalIp = config.get('externalIp');

  const cloned = lodash.cloneDeep(config);

  // sanitize [password, apiKey, privateKey, externalIp] fields in the dashmate config
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'password' ? hideString(value) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'id' ? hideString(value) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'key' ? hideString(value) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'apiKey' ? hideString(value) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'privateKey' ? hideString(value) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' && field === 'externalIp' ? hideString(value) : value));

  // sanitize also usernames & external ip from the rest of the fields values
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' ? value.replaceAll(
    username,
    hideString(username),
  ) : value));
  obfuscateObjectRecursive(cloned, (field, value) => (typeof value === 'string' ? value.replaceAll(
    externalIp,
    hideString(externalIp),
  ) : value));

  return cloned;
}