const util = require('util');

const DaemonFactory = require('ipfsd-ctl');
const IPFSApi = require('ipfs-api');

const df = DaemonFactory.create();

const spawn = util.promisify(df.spawn).bind(df);

// Workaround for https://github.com/ipfs/js-ipfsd-ctl/issues/202
process.execArgv = [];

/**
 * Start and stop IPFS instance for mocha test
 *
 * @return {Promise<IPFSApi>}
 */
module.exports = function startIPFSInstance() {
  let stop;
  let cleanup;

  return new Promise(((resolve) => {
    before(async function before() {
      this.timeout(20 * 1000); // slow Ctl

      const ipfsd = await spawn();
      stop = util.promisify(ipfsd.stop).bind(ipfsd);
      cleanup = util.promisify(ipfsd.cleanup).bind(ipfsd);

      resolve(new IPFSApi(ipfsd.apiAddr));
    });

    afterEach(async () => {
      if (cleanup) {
        await cleanup();
      }
    });

    after(async () => {
      if (stop) {
        await stop();
      }
    });
  }));
};
