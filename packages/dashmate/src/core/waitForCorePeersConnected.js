const wait = require('../util/wait');

/**
 * Wait Core to connect to peers
 *
 * @typedef {waitForCorePeersConnected}
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
async function waitForCorePeersConnected(rpcClient) {
  let hasPeers = false;

  do {
    const { result: peers } = await rpcClient.getPeerInfo();

    hasPeers = peers && peers.length > 0;

    if (!hasPeers) {
      await wait(10000);
    }
  } while (!hasPeers);
}

module.exports = waitForCorePeersConnected;
