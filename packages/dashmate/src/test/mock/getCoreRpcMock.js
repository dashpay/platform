/**
 * Core RPC client whose stubs answer from `core`, a plain object a test
 * mutates to move the chain or change DKG state between reads.
 * `core.beforeDkgInfo` and `core.afterDkgStatus`, when set, run right
 * before `quorum dkginfo` is answered and right after `quorum dkgstatus` is.
 * The tip hash is derived from the height unless `core.hash` is set.
 *
 * @param {Sinon} sinon
 * @param {{
 *   version: number,
 *   height: number,
 *   hash?: string,
 *   dkgInfo: Object,
 *   dkgStatus: Object,
 *   isSyncing?: boolean,
 *   dkgStatusDelayMs?: number,
 *   beforeDkgInfo?: Function,
 *   afterDkgStatus?: Function,
 * }} core
 */
export default function getCoreRpcMock(sinon, core) {
  return {
    getNetworkInfo: sinon.stub().callsFake(async () => ({ result: { version: core.version } })),
    getBlockchainInfo: sinon.stub().callsFake(async () => ({
      result: {
        blocks: core.height,
        bestblockhash: core.hash || `hash-${core.height}`,
        initialblockdownload: core.isSyncing === true,
      },
    })),
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

        const result = core.dkgStatus;

        if (core.afterDkgStatus) {
          core.afterDkgStatus();
        }

        return { result };
      }

      throw new Error(`unexpected quorum ${command}`);
    }),
  };
}
