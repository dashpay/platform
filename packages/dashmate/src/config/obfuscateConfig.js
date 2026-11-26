import lodash from 'lodash';
import obfuscateObjectRecursive from '../util/obfuscateObjectRecursive.js';
import Config from './Config.js';
import hideString from '../util/hideString.js';

/**
 * Config fields whose values are secret, at whatever depth they occur
 */
export const SECRET_FIELD_NAMES = ['password', 'key', 'apiKey', 'privateKey'];

/**
 * Build arguments that only choose how an image is built. Any other build
 * argument can carry a credential - a proxy URL with a password in it, a token
 * for a private registry - so its value is treated as secret.
 */
const NON_SECRET_BUILD_ARGS = ['CARGO_BUILD_PROFILE', 'SDK_TEST_DATA'];

/**
 * The credentials a URL written inside other text carries before its host:
 * `scheme://<userinfo>@host`. Matched up to the last `@` before the host,
 * because that is where URL parsers split, so a password can hold one
 * unescaped.
 */
const URL_USERINFO = /([a-z][a-z0-9+.-]*:[\\/]+)([^\s/\\?#]+)@/gi;

/**
 * @param {string} field
 * @param {string|undefined} parentField
 * @param {string|undefined} grandparentField
 * @return {boolean}
 */
function isSecretField(field, parentField, grandparentField) {
  if (SECRET_FIELD_NAMES.includes(field)) {
    return true;
  }

  // Build arguments are recognised where the schema puts them, in a build
  // block. Other things can be named `buildArgs` too - a Drive logger can be
  // named anything - and their values are ones the config has to keep.
  return parentField === 'buildArgs'
    && grandparentField === 'build'
    && !NON_SECRET_BUILD_ARGS.includes(field);
}

/**
 * @param {Object} object
 * @param {function(Object, string, boolean)} callback - called with the object
 *   holding each string value, its field, and whether that field is secret
 * @param {string} [parentField]
 * @param {string} [grandparentField]
 */
function forEachStringValue(object, callback, parentField = undefined, grandparentField = undefined) {
  for (const [field, value] of Object.entries(object)) {
    if (typeof value === 'object' && value !== null) {
      forEachStringValue(value, callback, field, parentField);
    } else if (typeof value === 'string') {
      callback(object, field, isSecretField(field, parentField, grandparentField));
    }
  }
}

/**
 * @param {string} value
 * @return {URL|null} the value parsed as a URL, when it is one with credentials
 */
function parseUrlWithCredentials(value) {
  let url;

  try {
    url = new URL(value);
  } catch {
    return null;
  }

  return url.username || url.password ? url : null;
}

/**
 * @param {string} text
 * @return {string}
 */
function decodeUrlComponent(text) {
  try {
    return decodeURIComponent(text);
  } catch {
    return text;
  }
}

/**
 * @param {string} value
 * @return {string}
 */
function hideUrlCredentials(value) {
  let hidden = value;

  // A value that is a URL is read the way a URL parser reads it, so every
  // spelling the parser accepts - extra slashes, an unescaped @ - is covered
  const url = parseUrlWithCredentials(value);

  if (url) {
    url.username = hideString(url.username);
    url.password = hideString(url.password);

    hidden = url.href;
  }

  // URLs inside the rest of it, a query parameter of that URL included. The
  // user name and the password are hidden apart, as the parser keeps them.
  return hidden.replace(URL_USERINFO, (_match, scheme, userinfo) => {
    const separatorIndex = userinfo.indexOf(':');

    const hiddenUserinfo = separatorIndex === -1
      ? hideString(userinfo)
      : `${hideString(userinfo.slice(0, separatorIndex))}:${hideString(userinfo.slice(separatorIndex + 1))}`;

    return `${scheme}${hiddenUserinfo}@`;
  });
}

/**
 * @param {string} value
 * @return {string[]} the credentials of the URLs in the value, in each form
 *   text can quote them in
 */
function getUrlCredentials(value) {
  const credentials = [];

  for (const [, , userinfo] of value.matchAll(URL_USERINFO)) {
    credentials.push(userinfo);

    // Text that mentions the password on its own, without the user name, and
    // as it is used rather than as the URL escapes it
    const separatorIndex = userinfo.indexOf(':');

    if (separatorIndex !== -1) {
      const password = userinfo.slice(separatorIndex + 1);

      credentials.push(password, decodeUrlComponent(password));
    }
  }

  const url = parseUrlWithCredentials(value);

  if (url?.password) {
    credentials.push(url.password, decodeUrlComponent(url.password));
  } else if (url?.username) {
    // A user name with no password is how a URL usually carries a token
    credentials.push(url.username, decodeUrlComponent(url.username));
  }

  return credentials;
}

/**
 * The values of every secret field in a config, and the credentials embedded
 * in any URL it holds, so they can be removed from text the config itself is
 * not part of
 *
 * @param {Config} config - as stored, not obfuscated
 * @return {string[]}
 */
export function getConfigSecretValues(config) {
  const values = new Set();

  forEachStringValue(config.getOptions(), (object, field, isSecret) => {
    const value = object[field];

    if (isSecret) {
      values.add(value);
    }

    // Secret values included: a proxy URL in a build argument is hidden whole
    // in the config copy, but a log line can still name its password alone
    getUrlCredentials(value).forEach((credential) => values.add(credential));
  });

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

  forEachStringValue(clonedOptions, (object, field, isSecret) => {
    // eslint-disable-next-line no-param-reassign
    object[field] = isSecret ? hideString(object[field]) : hideUrlCredentials(object[field]);
  });

  // sanitize also usernames & external ip from the rest of the fields values
  obfuscateObjectRecursive(clonedOptions, (_field, value) => (typeof value === 'string' ? value.replaceAll(
    username,
    hideString(username),
  ) : value));

  return new Config(config.getName(), clonedOptions);
}
