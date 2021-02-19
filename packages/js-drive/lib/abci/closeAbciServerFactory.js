const { promisify } = require('util');

/**
 *
 * @param {net.Server} abciServer
 * @return {closeAbciServer}
 */
function closeAbciServerFactory(abciServer) {
  /**
   * @typedef {closeAbciServer}
   * @return {Promise<void>}
   */
  async function closeAbciServer() {
    if (!abciServer.listening) {
      return;
    }

    const close = promisify(abciServer.close.bind(abciServer));
    await close();
  }

  return closeAbciServer;
}

module.exports = closeAbciServerFactory;
