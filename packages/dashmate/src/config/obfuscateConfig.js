import lodash from 'lodash';
import Config from './Config.js';
import hideString from '../util/hideString.js';
import { getOptionValue, hideOptionValue, isOption } from '../util/commandLineOption.js';
import maskOperatorIdentity, { getOperatorIdentity } from '../util/maskOperatorIdentity.js';

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
 * `scheme://<userinfo>@host`, or `scheme:<userinfo>@host` for the web schemes
 * a URL parser also takes without slashes. Matched up to the last `@` before
 * the host, because that is where URL parsers split, so a password can hold
 * one unescaped.
 */
const URL_USERINFO = /((?:[a-z][a-z0-9+.-]*:[\\/]+)|(?:(?:https?|wss?|ftp):))([^\s/\\?#]+)@/gi;

/**
 * A URL written inside other text with credentials before its host, up to the
 * start of its query, and the query itself
 */
const URL_USERINFO_QUERY = new RegExp(`((?:${URL_USERINFO.source})[^\\s?#]*\\?)([^\\s#]*)`, 'gi');

const VALUE_KIND = {
  PLAIN: 'plain',
  // Hidden whole
  SECRET: 'secret',
  // An argument for a service's own command line, where an option can carry a
  // key or a password as `-name=value`: the value is hidden, the name kept.
  // Core reads values only after `=` and refuses a bare argument, so one
  // that is not an option is a value meant for the option before it and is
  // hidden whole.
  COMMAND_ARGUMENT: 'commandArgument',
};

/**
 * @param {string} field
 * @param {string|undefined} parentField
 * @param {string|undefined} grandparentField
 * @return {string} one of VALUE_KIND
 */
function classifyValue(field, parentField, grandparentField) {
  if (SECRET_FIELD_NAMES.includes(field)) {
    return VALUE_KIND.SECRET;
  }

  // Build arguments and command arguments are recognised where the schema
  // puts them. Other things can carry the same names - a Drive logger can be
  // named anything - and their values are ones the config has to keep.
  if (parentField === 'buildArgs' && grandparentField === 'build') {
    return NON_SECRET_BUILD_ARGS.includes(field) ? VALUE_KIND.PLAIN : VALUE_KIND.SECRET;
  }

  if (parentField === 'commandArgs' && grandparentField === 'docker') {
    return VALUE_KIND.COMMAND_ARGUMENT;
  }

  return VALUE_KIND.PLAIN;
}

/**
 * @param {Object} object
 * @param {function(Object, string, string)} callback - called with the object
 *   holding each string value, its field, and its VALUE_KIND
 * @param {string} [parentField]
 * @param {string} [grandparentField]
 */
function forEachStringValue(object, callback, parentField = undefined, grandparentField = undefined) {
  for (const [field, value] of Object.entries(object)) {
    if (typeof value === 'object' && value !== null) {
      forEachStringValue(value, callback, field, parentField);
    } else if (typeof value === 'string') {
      callback(object, field, classifyValue(field, parentField, grandparentField));
    }
  }
}

/**
 * Config fields whose values are free text an operator writes, and nothing
 * reads as a value
 */
const FREE_TEXT_FIELD_NAMES = ['description', 'email'];

/**
 * @param {string} value
 * @return {boolean} whether the value is an absolute local path
 */
function isLocalPath(value) {
  return value.startsWith('/') || /^[A-Za-z]:[\\/]/.test(value);
}

/**
 * How many URLs deep, through query values, credentials are looked for
 */
const MAX_URL_NESTING = 3;

/**
 * @param {string} value
 * @return {URL|null} the value parsed as a URL, when it is one
 */
function parseUrl(value) {
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

/**
 * @param {URL} url
 * @return {string[]} the URL's query values, decoded: a URL passed as one is
 *   percent-encoded, so its credentials only show once it is
 */
function getDecodedQueryValues(url) {
  return [...url.searchParams.values()];
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
 * @param {string} text
 * @return {string} the text with the credentials of the URLs written in it
 *   hidden
 */
function hideTextUrlCredentials(text) {
  // The user name and the password are hidden apart, as the parser keeps them
  return text.replace(URL_USERINFO, (_match, scheme, userinfo) => {
    const separatorIndex = userinfo.indexOf(':');

    const hiddenUserinfo = separatorIndex === -1
      ? hideString(userinfo)
      : `${hideString(userinfo.slice(0, separatorIndex))}:${hideString(userinfo.slice(separatorIndex + 1))}`;

    return `${scheme}${hiddenUserinfo}@`;
  });
}

/**
 * @param {string} text
 * @return {string} the text with the query values hidden in each URL written
 *   in it with credentials: a URL that carries a password can carry a token
 *   in its query too
 */
function hideTextUrlQueryValues(text) {
  return text.replace(URL_USERINFO_QUERY, (_match, start, _scheme, _userinfo, query) => {
    const hiddenQuery = query.split('&').map((parameter) => {
      const separatorIndex = parameter.indexOf('=');

      return separatorIndex === -1
        ? parameter
        : `${parameter.slice(0, separatorIndex + 1)}${hideString(parameter.slice(separatorIndex + 1))}`;
    });

    return `${start}${hiddenQuery.join('&')}`;
  });
}

/**
 * @param {string} value
 * @param {number} [nesting] - how many query values deep the value is
 * @return {string}
 */
function hideUrlCredentials(value, nesting = 0) {
  let hidden = value;

  // A value that is a URL is read the way a URL parser reads it, so every
  // spelling the parser accepts - extra slashes, an unescaped @ - is covered
  const url = nesting < MAX_URL_NESTING ? parseUrl(value) : null;

  if (url) {
    let changed = false;

    if (url.username || url.password) {
      url.username = hideString(url.username);
      url.password = hideString(url.password);

      changed = true;
    }

    const query = [...url.searchParams];
    const hiddenQuery = query.map(([name, queryValue]) => [
      name,
      hideUrlCredentials(queryValue, nesting + 1),
    ]);

    if (hiddenQuery.some(([, queryValue], index) => queryValue !== query[index][1])) {
      url.search = new URLSearchParams(hiddenQuery).toString();

      changed = true;
    }

    // Written back only when something was hidden, so a URL with nothing to
    // hide stays exactly as the operator wrote it
    if (changed) {
      hidden = url.href;
    }
  }

  // URLs inside the rest of it, a query parameter written out as is included
  return hideTextUrlCredentials(hidden);
}

/**
 * @param {string} userinfo - `user`, or `user:password`, as a URL writes it
 * @return {string[]} the whole of it and each part, as written and decoded,
 *   since text can quote any of them on its own
 */
function getUserinfoParts(userinfo) {
  const separatorIndex = userinfo.indexOf(':');

  const parts = separatorIndex === -1
    ? [userinfo]
    : [userinfo, userinfo.slice(0, separatorIndex), userinfo.slice(separatorIndex + 1)];

  return parts.flatMap((part) => [part, decodeUrlComponent(part)]);
}

/**
 * @param {string} value
 * @param {number} [nesting] - how many query values deep the value is
 * @return {string[]} the credentials of the URLs in the value, in each form
 *   text can quote them in
 */
function getUrlCredentials(value, nesting = 0) {
  const credentials = [];

  for (const [, , userinfo] of value.matchAll(URL_USERINFO)) {
    credentials.push(...getUserinfoParts(userinfo));
  }

  const url = nesting < MAX_URL_NESTING ? parseUrl(value) : null;

  if (url) {
    // The parser also reads URLs the pattern does not, such as one with no
    // slashes after the scheme
    if (url.username || url.password) {
      credentials.push(...getUserinfoParts(url.password ? `${url.username}:${url.password}` : url.username));
    }

    // A URL in a query value is read decoded, and its credentials are also
    // looked for as the outer URL writes them, encoded once more: by
    // encodeURIComponent, or by form encoding as URLSearchParams does, which
    // escapes more
    for (const queryValue of getDecodedQueryValues(url)) {
      for (const credential of getUrlCredentials(queryValue, nesting + 1)) {
        credentials.push(
          credential,
          encodeURIComponent(credential),
          new URLSearchParams([['', credential]]).toString().slice(1),
        );
      }
    }
  }

  return credentials.filter((credential) => credential.length > 0);
}

/**
 * The values of every secret field in a config, of every option it passes on
 * a service's command line, and the credentials embedded in any URL it holds,
 * so they can be removed from text the config itself is not part of
 *
 * @param {Config} config - as stored, not obfuscated
 * @return {string[]}
 */
export function getConfigSecretValues(config) {
  const values = new Set();

  forEachStringValue(config.getOptions(), (object, field, kind) => {
    const value = object[field];

    if (kind === VALUE_KIND.SECRET) {
      values.add(value);
    }

    const argumentValue = isOption(value) ? getOptionValue(value) : value;

    if (kind === VALUE_KIND.COMMAND_ARGUMENT && argumentValue !== null) {
      values.add(argumentValue);
    }

    // Secret values included: a proxy URL in a build argument is hidden whole
    // in the config copy, but a log line can still name its password alone
    getUrlCredentials(value).forEach((credential) => values.add(credential));
  });

  return [...values];
}

/**
 * @param {Config} config
 * @param {function(string): string} [scrubFreeText] - masks secrets known
 *   from elsewhere in free text the operator writes, such as the node
 *   description. No other value is passed to it: the copy has to keep the
 *   values the config is made of, which a secret can be spelled like.
 * @return {Config}
 */
export default function obfuscateConfig(
  config,
  scrubFreeText = (text) => text,
) {
  const identity = getOperatorIdentity();

  const clonedOptions = lodash.cloneDeep(config.getOptions());

  forEachStringValue(clonedOptions, (object, field, kind) => {
    let value = object[field];

    if (kind === VALUE_KIND.SECRET) {
      value = hideString(value);
    } else {
      if (kind === VALUE_KIND.COMMAND_ARGUMENT) {
        value = isOption(value) ? hideOptionValue(value, hideString) : hideString(value);
      }

      value = hideUrlCredentials(value);
    }

    // The operator's name and home directory are masked the way they are in
    // the samples, except that a home directory is written `/~`: the schema
    // requires log paths to be absolute, and doctor validates this copy when
    // it reads a report back. The account name is masked where it can name
    // the operator - in paths and in free text - but not in values the
    // config is made of, which it can be spelled like: an account named
    // `info` or `production` would otherwise change a log level or the
    // environment, or make the copy invalid.
    const isFreeText = FREE_TEXT_FIELD_NAMES.includes(field);

    // Before the operator's identity is masked, which could rewrite part of
    // a secret. Nothing reads free text as a value, so a URL's query in it
    // can be hidden too: once its credentials are hidden, a secret the query
    // shares with a config value no longer matches that value whole.
    if (isFreeText) {
      value = scrubFreeText(hideTextUrlQueryValues(value));
    }

    const namesOperator = isLocalPath(value) || isFreeText;

    value = maskOperatorIdentity(
      value,
      namesOperator ? identity : { ...identity, username: null },
      { homeReplacement: '/~' },
    );

    // eslint-disable-next-line no-param-reassign
    object[field] = value;
  });

  return new Config(config.getName(), clonedOptions);
}
