import wait from '../util/wait.js';

/**
 * Wait Core to connect to peers
 *
 * @typedef {waitForCorePeersConnected}
 * @param {RpcClient} rpcClient
 * @return {Promise<void>}
 */
export default async function waitForCorePeersConnected(rpcClient) {
  let hasPeers = false;

  do {
    const { result: peers } = await rpcClient.getPeerInfo();

    hasPeers = peers && peers.length > 0;

    if (!hasPeers) {
      await wait(1000);
    }
  } while (!hasPeers);
}
