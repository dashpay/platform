function isCamelCase(str) {
  return !!str.match(/^[a-z]+[A-Z]/);
}

function camelToSnakeCase(str) {
  if (isCamelCase(str)) {
    return str.replace(/[A-Z]/g, '_$&');
  }

  return str;
}

/**
 * @param {Object} envs
 * @param {*} value
 * @param {string} [keyPrefix='']
 */
function buildEnvs(envs, value, keyPrefix = '') {
  if (typeof value === 'object' && value !== null) {
    if (keyPrefix.length > 0) {
      // eslint-disable-next-line no-param-reassign
      keyPrefix += '_';
    }

    for (const [k, v] of Object.entries(value)) {
      let key = k;

      if (k === '*') {
        key = '_';
      }

      buildEnvs(
        envs,
        v,
        `${keyPrefix}${camelToSnakeCase(key).toUpperCase()}`,
      );
    }
  } else {
    if (value === null || value === undefined) {
      // eslint-disable-next-line no-param-reassign
      value = '';
    }

    // eslint-disable-next-line no-param-reassign
    envs[keyPrefix] = value.toString();
  }
}

/**
 * @param {Object} object
 * @returns {Object}
 */
function convertObjectToEnvs(object) {
  const envs = {};

  buildEnvs(envs, object);

  return envs;
}

module.exports = convertObjectToEnvs;
