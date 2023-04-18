const os = require('os');
const publicIp = require('public-ip');
const prettyMs = require('pretty-ms');
const prettyByte = require('pretty-bytes');

/**
 * @returns {getHostScope}
 */
function getHostScopeFactory() {
  /**
   * Get host status scope
   *
   * @typedef {Promise} getHostScope
   * @returns {Promise<Object>}
   */
  async function getHostScope() {
    return ({
      hostname: os.hostname(),
      uptime: prettyMs(os.uptime() * 1000),
      platform: os.platform(),
      arch: os.arch(),
      username: os.userInfo().username,
      memory: `${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`,
      cpus: os.cpus().length,
      ip: await publicIp.v4(),
    });
  }

  return getHostScope;
}

module.exports = getHostScopeFactory;
