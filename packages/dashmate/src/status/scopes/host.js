import os from 'os';
import { publicIp } from 'public-ip';
import prettyMs from 'pretty-ms';
import prettyByte from 'pretty-bytes';

/**
 * @returns {getHostScope}
 */
export function getHostScopeFactory() {
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
      scope.ip = await publicIp.v4();
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
