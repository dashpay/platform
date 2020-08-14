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
 * @param {string} [key='']
 */
function buildEnvs(envs, value, key = '') {
  if (typeof value === 'object' && value !== null) {
    if (key.length > 0) {
      // eslint-disable-next-line no-param-reassign
      key += '_';
    }

    for (const [k, v] of Object.entries(value)) {
      buildEnvs(
        envs,
        v,
        `${key}${camelToSnakeCase(k).toUpperCase()}`,
      );
    }
  } else {
    if (value === null || value === undefined) {
      // eslint-disable-next-line no-param-reassign
      value = '';
    }

    // eslint-disable-next-line no-param-reassign
    envs[key] = value.toString();
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
