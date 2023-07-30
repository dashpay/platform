const os = require('os');
const prettyMs = require('pretty-ms');
const prettyByte = require('pretty-bytes');

/**
 * @returns {getHostScope}
 */
function getHostScopeFactory(resolvePublicIpV4) {
  /**
   * Get host status scope
   *
   * @typedef {Promise} getHostScope
   * @returns {Promise<Object>}
   */
  async function getHostScope() {
    const scope = {
      hostname: null,
      uptime: null,
      platform: null,
      arch: null,
      username: null,
      memory: null,
      cpus: null,
      ip: null,
    };

    try {
      scope.hostname = os.hostname();
      scope.uptime = prettyMs(os.uptime() * 1000);
      scope.platform = os.platform();
      scope.arch = os.arch();
      scope.username = os.userInfo().username;
      scope.memory = `${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`;
      scope.cpus = os.cpus().length;
      scope.ip = await resolvePublicIpV4()
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Could not retrieve host scope', e);
      }
    }

    return scope;
  }

  return getHostScope;
}

module.exports = getHostScopeFactory;
