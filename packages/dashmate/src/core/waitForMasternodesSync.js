import wait from '../util/wait.js';

/**
 * Wait Core to be synced
 *
 * @typedef {waitForCoreSync}
 * @param {RpcClient} rpcClient
 * @param {function(progress: number)} [progressCallback]
 * @return {Promise<void>}
 */
export default async function waitForMasternodesSync(rpcClient, progressCallback = () => {}) {
  let isSynced = false;
  let verificationProgress = 0.0;

  do {
    try {
      await rpcClient.mnsync('next');

      ({
        result: { IsSynced: isSynced },
      } = await rpcClient.mnsync('status'));

      ({
        result: { verificationprogress: verificationProgress },
      } = await rpcClient.getBlockchainInfo());
    } catch (e) {
      // Core RPC is not started yet
      if (!e.message.includes('Dash JSON-RPC: Request Error: ') && !e.message.includes('Timeout') && e.code !== -28) {
        throw e;
      }

      progressCallback(verificationProgress);

      // Wait for Core RPC is started
      await wait(50);

      continue;
    }

    if (!isSynced) {
      progressCallback(verificationProgress);

      await wait(300);
    }
  } while (!isSynced);
}
