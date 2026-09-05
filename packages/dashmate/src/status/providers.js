import https from 'https';
import semver from 'semver';

const MAX_REQUEST_TIMEOUT = 5000;
const MAX_RESPONSE_SIZE = 1 * 1024 * 1024; // 1 MB

// A remote version string is printed to the operator's terminal, included in JSON
// output and passed to package managers, so only a strict semver shape is accepted.
// The anchors and character classes leave no room for control or ANSI escape
// characters, nor for the specifiers a package manager would treat as a location to
// install from (git+https://…, file:…, https://….tgz, npm: aliases).
const VERSION_REGEX = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;

const MAX_LOG_LENGTH = 200;

// Characters that let remote text hijack a terminal line or disguise itself in the
// output: C0 and C1 controls (including the escape that opens an ANSI sequence), zero
// width characters, line separators, and the bidirectional overrides and isolates that
// reorder what an operator reads. Matching them is the point, hence the disabled rule.
// eslint-disable-next-line no-control-regex
const UNSAFE_LOG_CHARACTERS_REGEX = /[\u0000-\u001f\u007f-\u009f\u200b-\u200f\u2028-\u2029\u202a-\u202e\u2066-\u2069\ufeff]/g;

/**
 * Make text received from a remote host safe to print
 *
 * Error messages quote the payload that failed to parse, so remote bytes reach the
 * terminal through diagnostics even when the value itself is rejected. Both the
 * dangerous characters and the length are bounded here.
 *
 * @param {*} text
 * @returns {string}
 */
const sanitizeForLog = (text) => (typeof text === 'string'
  ? text.replace(UNSAFE_LOG_CHARACTERS_REGEX, '').slice(0, MAX_LOG_LENGTH)
  : '[unprintable]');

const request = async (url, options = {}) => {
  try {
    return await fetch(url, {
      ...options,
      signal: AbortSignal.timeout(MAX_REQUEST_TIMEOUT),
    });
  } catch (e) {
    // Every transport failure (DNS, connection reset, timeout, abort) is reported as
    // an unknown result. Callers use these providers to enrich output, so an
    // unreachable remote host must never fail the command that called them.
    if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn(`Could not fetch ${url}: ${e.name}: ${sanitizeForLog(e.message)}`);
    }

    return null;
  }
};

/**
 * Read a response body, giving up as soon as it exceeds the size limit
 *
 * @param {Response} response
 * @returns {Promise<string|null>} body text, or null if it is too big to read
 */
const readCappedBody = async (response) => {
  const declaredSize = Number(response.headers.get('content-length'));

  if (Number.isFinite(declaredSize) && declaredSize > MAX_RESPONSE_SIZE) {
    if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn(`Response of ${declaredSize} bytes exceeds the size limit`);
    }

    // The body is never read on this path, and until it is cancelled the connection
    // stays checked out of the pool
    response.body?.cancel().catch(() => {});

    return null;
  }

  if (!response.body) {
    return null;
  }

  const chunks = [];
  let size = 0;

  try {
    // The declared size is only a hint, so the body is also measured while it is read
    // and the connection is dropped before an unbounded response can exhaust memory
    for await (const chunk of response.body) {
      size += chunk.length;

      if (size > MAX_RESPONSE_SIZE) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn('Response size exceeded');
        }

        return null;
      }

      chunks.push(chunk);
    }
  } catch (e) {
    // The connection can still drop after the headers arrived, leaving a truncated body
    if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn(`Could not read response: ${sanitizeForLog(e.message)}`);
    }

    return null;
  }

  return Buffer.concat(chunks).toString('utf8');
};

const requestJSON = async (url, options = {}) => {
  const response = await request(url, options);

  if (!response) {
    return null;
  }

  // Error responses, including GitHub's 403 when the unauthenticated rate limit is
  // hit, carry no usable data and are indistinguishable from an unreachable host
  if (!response.ok) {
    if (process.env.DEBUG) {
      // eslint-disable-next-line no-console
      console.warn(`Request to ${url} failed with status code ${response.status}`);
    }

    // The body is never read on this path, and until it is cancelled the connection
    // stays checked out of the pool
    response.body?.cancel().catch(() => {});

    return null;
  }

  const body = await readCappedBody(response);

  if (body === null) {
    return null;
  }

  try {
    return JSON.parse(body);
  } catch (e) {
    if (process.env.DEBUG) {
      // The parser quotes an excerpt of the payload it choked on, so this message
      // carries remote bytes and cannot be printed as it stands
      // eslint-disable-next-line no-console
      console.warn(`Could not parse response from ${url}: ${sanitizeForLog(e.message)}`);
    }

    return null;
  }
};

/**
 * Extract a version from a release tag name, rejecting anything that is not a version
 *
 * The tag name is arbitrary text chosen by whoever cut the release, so it is validated
 * here, at the boundary, before it can be stored, printed or handed to a package
 * manager.
 *
 * @param {*} tagName
 * @returns {string|null} version, or null if the tag does not name one
 */
const parseVersionFromTagName = (tagName) => {
  if (typeof tagName !== 'string') {
    return null;
  }

  // Release tags are conventionally prefixed with "v", but the prefix is optional
  const version = tagName.startsWith('v') ? tagName.slice(1) : tagName;

  // semver rejects what the shape check cannot, such as leading zeroes in "01.2.3",
  // and normalizes the result: build metadata is dropped, because version comparison
  // ignores it while a package manager would refuse to resolve a version carrying it
  const normalizedVersion = VERSION_REGEX.test(version) ? semver.valid(version) : null;

  if (normalizedVersion === null && process.env.DEBUG) {
    // eslint-disable-next-line no-console
    console.warn(`Ignoring release tag that is not a version: ${sanitizeForLog(tagName)}`);
  }

  return normalizedVersion;
};

const insightURLs = {
  testnet: 'https://testnet-insight.dashevo.org/insight-api',
  mainnet: 'https://insight.dash.org/insight-api',
};

export default {
  insight: (chain) => ({
    /**
     * Get the status of an insight instance.
     *
     * @returns {Promise<object|null>} A promise that resolves to the status, or to null
     * when it cannot be determined. A host that answers with something other than a
     * status, such as a maintenance or CDN error page served with a 200, counts as
     * undetermined: callers read the block height without re-checking its type.
     */
    status: async () => {
      if (!insightURLs[chain]) {
        return null;
      }

      const json = await requestJSON(`${insightURLs[chain]}/status`);

      // Requiring the one field callers use, with the type they expect, keeps text
      // chosen by the remote host from reaching the terminal and the JSON output
      if (!Number.isInteger(json?.info?.blocks) || json.info.blocks < 0) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.warn(`Insight ${chain} did not report a block height`);
        }

        return null;
      }

      return json;
    },
  }),
  github: {
    /**
     * Get the version of the latest release of a GitHub repository.
     *
     * GitHub reports the most recently *published* release, which is not necessarily
     * the highest version: a patch back-ported to an older branch and published after
     * a newer release is reported here. A caller that acts on this version, rather
     * than only displaying it, must compare it against the version it already has,
     * or it can walk backwards onto an older release.
     *
     * @param {string} repoSlug - The owner and name of the repository.
     * @returns {Promise<string|null>} A promise that resolves to the version, or to
     * null when it cannot be determined, including when the host is unreachable, the
     * API rate limit is exhausted, or the release is not tagged with a version.
     */
    release: async (repoSlug) => {
      const headers = {};

      // Unauthenticated requests share a per-IP rate limit, which a fleet behind one
      // address exhausts quickly, so authenticate when a token is available. Tokens
      // are commonly read from a file, and the trailing newline that comes with them
      // is an illegal header value that would fail the request instead
      const token = process.env.GITHUB_TOKEN?.trim();

      if (token) {
        headers.Authorization = `Bearer ${token}`;
      }

      const json = await requestJSON(
        `https://api.github.com/repos/${repoSlug}/releases/latest`,
        { headers },
      );

      if (!json) {
        return null;
      }

      return parseVersionFromTagName(json.tag_name);
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

            const error = new Error(`Invalid status code ${res.statusCode}`);

            res.destroy(error);

            // Do not handle request further
            return;
          }

          // Optionally set the encoding to receive strings directly
          res.setEncoding('utf8');

          // Collect data chunks
          res.on('data', (chunk) => {
            data += chunk;

            if (data.length > MAX_RESPONSE_SIZE) {
              if (process.env.DEBUG) {
                // eslint-disable-next-line no-console
                console.warn('Port check response size exceeded');
              }

              const error = new Error('Response size exceeded');

              req.destroy(error);
            }
          });

          // Handle the end of the response
          res.on('end', () => {
            resolve(data);
          });
        });

        req.setTimeout(MAX_REQUEST_TIMEOUT, () => {
          const error = new Error('Port check timed out');

          req.destroy(error);
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
