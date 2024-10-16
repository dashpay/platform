import https from 'https';
import { PortStateEnum } from './enums/portState.js';

const MAX_REQUEST_TIMEOUT = 5000;
const MAX_RESPONSE_SIZE = 1 * 1024 * 1024; // 1 MB

const request = async (url) => {
  try {
    return await fetch(url, {
      signal: AbortSignal.timeout(MAX_REQUEST_TIMEOUT),
    });
  } catch (e) {
    if (e.name === 'FetchError' || e.name === 'AbortError') {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.warn(`Could not fetch: ${e}`);
      }
      return null;
    }
    throw e;
  }
};

const requestJSON = async (url) => {
  const response = await request(url);

  if (response) {
    return response.json();
  }

  return response;
};

const insightURLs = {
  testnet: 'https://testnet-insight.dashevo.org/insight-api',
  mainnet: 'https://insight.dash.org/insight-api',
};

export default {
  insight: (chain) => ({
    status: async () => {
      if (!insightURLs[chain]) {
        return null;
      }

      return requestJSON(`${insightURLs[chain]}/status`);
    },
  }),
  github: {
    release: async (repoSlug) => {
      const json = await requestJSON(`https://api.github.com/repos/${repoSlug}/releases/latest`);

      if (json.message) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Github API: ${json.message}`);
        }

        return null;
      }

      return json.tag_name.substring(1);
    },
  },
  mnowatch: {
    /**
     * Check the status of a port and optionally validate an IP address.
     *
     * @param {number} port - The port number to check.
     * @param {string} [ip] - Optional. The IP address to validate.
     * @returns {Promise<string>} A promise that resolves to the port status.
     */
    checkPortStatus: async (port, ip = undefined) => {
      // We use http request instead fetch function to force
      // using IPv4 otherwise mnwatch could try to connect to IPv6 node address
      // and fail (Core listens for IPv4 only)
      // https://github.com/dashpay/platform/issues/2100

      const options = {
        hostname: 'mnowatch.org',
        port: 443,
        path: ip ? `/${port}/?validateIp=${ip}` : `/${port}/`,
        method: 'GET',
        family: 4, // Force IPv4
        timeout: MAX_REQUEST_TIMEOUT,
      };

      return new Promise((resolve, reject) => {
        const req = https.request(options, (res) => {
          let data = '';

          // Check if the status code is 200
          if (res.statusCode !== 200) {
            if (process.env.DEBUG) {
              // eslint-disable-next-line no-console
              console.warn(`Port check request failed with status code ${res.statusCode}`);
            }
            // Consume response data to free up memory
            res.resume();
            reject(new Error(`Invalid status code ${res.statusCode}`));
            return;
          }

          // Optionally set the encoding to receive strings directly
          res.setEncoding('utf8');

          // Collect data chunks
          res.on('data', (chunk) => {
            data += chunk;

            if (data.length > MAX_RESPONSE_SIZE) {
              reject(new Error('Response size exceeded'));

              if (process.env.DEBUG) {
                // eslint-disable-next-line no-console
                console.warn('Port check response size exceeded');
              }

              req.destroy();
            }
          });

          // Handle the end of the response
          res.on('end', () => {
            resolve(data);
          });
        });

        req.on('error', (e) => {
          if (process.env.DEBUG) {
            // eslint-disable-next-line no-console
            console.warn(`Port check request failed: ${e}`);
          }

          reject(e);
        });

        req.end();
      });
    },
  },
};
