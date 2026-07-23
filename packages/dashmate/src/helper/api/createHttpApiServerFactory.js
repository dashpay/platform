import jayson from 'jayson/promise/index.js';

const STATUS_METHOD = 'status';
const ALLOWED_STATUS_PARAMETERS = new Set(['config', 'format']);
const SAFE_CONFIG_NAME_PATTERN = /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/;

/**
 * Convert the deliberately narrow helper API request into CLI arguments.
 *
 * @param {string} method
 * @param {Object} params
 * @returns {string[]|null}
 */
export function createHelperCommandArgs(method, params) {
  if (method !== STATUS_METHOD) {
    return null;
  }

  if (params === null || Array.isArray(params) || typeof params !== 'object') {
    throw new TypeError('Status parameters must be an object');
  }

  const parameterNames = Object.keys(params);
  if (parameterNames.some((name) => !ALLOWED_STATUS_PARAMETERS.has(name))) {
    throw new TypeError('Unsupported status parameter');
  }

  if (params.format !== 'json') {
    throw new TypeError('Status format must be json');
  }

  if (typeof params.config !== 'string' || !SAFE_CONFIG_NAME_PATTERN.test(params.config)) {
    throw new TypeError('Invalid config name');
  }

  return [STATUS_METHOD, `--format=${params.format}`, `--config=${params.config}`];
}

export default function createHttpApiServerFactory() {
  /**
   * @return {HttpServer}
   */
  function createHttpApiServer() {
    const server = new jayson.Server({}, {
      router(method, params) {
        let argv;

        try {
          argv = createHelperCommandArgs(method, params);
        } catch (error) {
          return new jayson.Method(async () => {
            throw server.error(-32602, error.message);
          });
        }

        if (argv === null) {
          return undefined;
        }

        return new jayson.Method(async () => {
          try {
            const { execute } = await import('@oclif/core');
            return await execute({ dir: import.meta.url, args: argv });
          } catch (error) {
            // Log the real failure for the operator; the client still only
            // sees the generic error so no internals leak over the API.
            console.error('Helper API status request failed:', error);
            throw server.error(-32603, 'Status request failed');
          }
        });
      },
    });

    return server.http();
  }

  return createHttpApiServer;
}
