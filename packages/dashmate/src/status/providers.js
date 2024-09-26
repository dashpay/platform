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
    checkPortStatus: async (port) => {
      // We use http request instead fetch function to force
      // using IPv4 otherwise mnwatch could try to connect to IPv6 node address
      // and fail (Core listens for IPv4 only)
      // https://github.com/dashpay/platform/issues/2100

      const options = {
        hostname: 'mnowatch.org',
        port: 443,
        path: `/${port}/`,
        method: 'GET',
        family: 4, // Force IPv4
      };

      return new Promise((resolve) => {
        const req = https.request(options, (res) => {
          let data = '';

          // Optionally set the encoding to receive strings directly
          res.setEncoding('utf8');

          res.setTimeout(MAX_REQUEST_TIMEOUT, () => {
            if (process.env.DEBUG) {
              // eslint-disable-next-line no-console
              console.warn(`Port check ${MAX_REQUEST_TIMEOUT} timeout reached`);
            }

            resolve(PortStateEnum.ERROR);

            req.destroy();
          });

          // Collect data chunks
          res.on('data', (chunk) => {
            data += chunk;

            if (data.length > MAX_RESPONSE_SIZE) {
              resolve(PortStateEnum.ERROR);

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
            console.warn(e);
          }

          resolve(PortStateEnum.ERROR);
        });

        req.setTimeout(MAX_REQUEST_TIMEOUT, () => {
          if (process.env.DEBUG) {
            // eslint-disable-next-line no-console
            console.warn(`Port check ${MAX_REQUEST_TIMEOUT} timeout reached`);
          }

          resolve(PortStateEnum.ERROR);

          req.destroy(); // Abort the request
        });

        req.end();
      });
    },
  },
};
