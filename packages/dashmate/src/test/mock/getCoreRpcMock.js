/**
 * Core RPC client whose stubs answer from `core`, a plain object a test
 * mutates to move the chain or change DKG state between reads.
 * `core.beforeDkgInfo` and `core.afterDkgStatus`, when set, run right
 * before `quorum dkginfo` is answered and right after `quorum dkgstatus` is.
 * Block hashes are derived from their height; `core.hash` overrides the tip's,
 * and `core.hashAt(height)`, when set, any block's. `quorum dkgstatus` lists
 * no quorum connections unless the test's `dkgStatus` does.
 *
 * @param {Sinon} sinon
 * @param {{
 *   version: number,
 *   height: number,
 *   hash?: string,
 *   hashAt?: Function,
 *   dkgInfo: Object,
 *   dkgStatus: Object,
 *   isSyncing?: boolean,
 *   dkgStatusDelayMs?: number,
 *   beforeDkgInfo?: Function,
 *   afterDkgStatus?: Function,
 * }} core
 */
export default function getCoreRpcMock(sinon, core) {
  const getHash = (height) => {
    if (core.hashAt) {
      return core.hashAt(height);
    }

    return height === core.height && core.hash ? core.hash : `hash-${height}`;
  };

  return {
    getNetworkInfo: sinon.stub().callsFake(async () => ({ result: { version: core.version } })),
    getBlockchainInfo: sinon.stub().callsFake(async () => ({
      result: {
        blocks: core.height,
        bestblockhash: getHash(core.height),
        initialblockdownload: core.isSyncing === true,
      },
    })),
    getBlockHash: sinon.stub().callsFake(async (height) => ({ result: getHash(height) })),
    quorum: sinon.stub().callsFake(async (command) => {
      if (command === 'dkginfo') {
        if (core.beforeDkgInfo) {
          core.beforeDkgInfo();
        }

        return { result: core.dkgInfo };
      }

      if (command === 'dkgstatus') {
        if (core.dkgStatusDelayMs) {
          await new Promise((resolve) => { setTimeout(resolve, core.dkgStatusDelayMs); });
        }

        const result = core.dkgStatus && typeof core.dkgStatus === 'object'
          ? { quorumConnections: [], ...core.dkgStatus }
          : core.dkgStatus;

        if (core.afterDkgStatus) {
          core.afterDkgStatus();
        }

        return { result };
      }

      throw new Error(`unexpected quorum ${command}`);
    }),
  };
}
