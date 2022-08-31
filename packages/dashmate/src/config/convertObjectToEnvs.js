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
    let keyPrefix = '';

    if (key !== '') {
      keyPrefix = `${key}_`;
    }

    for (const [k, v] of Object.entries(value)) {
      buildEnvs(
        envs,
        v,
        `${keyPrefix}${camelToSnakeCase(k).toUpperCase()}`,
      );
    }
  } else {
    let envValue = value;
    if (value === null || value === undefined) {
      envValue = '';
    }

    // Sanitize key
    const envKey = key.replace(/\W/g, '_');

    // eslint-disable-next-line no-param-reassign
    envs[envKey] = envValue.toString();
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
