import loadDpp from '@dashevo/wasm-dpp';
const { Identity } = loadDpp;
import GrpcErrorCodes from '@dashevo/grpc-common/lib/server/error/GrpcErrorCodes.js';

import Worker from '../Worker.js';

/**
 * @property {number} gapLimit
 */
class IdentitySyncWorker extends Worker {
  constructor(options) {
    super({
      name: 'IdentitySyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: 60 * 1000,
      awaitOnInjection: true,
      gapLimit: 10,
      dependencies: [
        'storage',
        'transport',
        'walletId',
        'identities',
      ],
      ...options,
    });
  }

  // eslint-disable-next-line
  async onStart() {
    // Load DPP to make sure Identity and decodeProtocolEntity are available.
    // wasm-dpp is CJS; under NodeNext the default import may resolve to the
    // module namespace object instead of the callable. Unwrap defensively.
    const load = loadDpp.default ?? loadDpp;
    await load();
  }

  async execute() {
    const walletStore = this.storage.getWalletStore(this.walletId);
    const indexedIds = await walletStore.getIndexedIdentityIds();

    // Add gaps to empty indices
    const unusedIndices = [];
    indexedIds.forEach((id, index) => {
      if (!id) {
        return;
      }

      unusedIndices.push(index);
    });

    let gapCount = 0;
    let unusedIndex;
    let index = -1;
    while (gapCount < this.gapLimit) {
      unusedIndex = unusedIndices.shift();

      // check unused indices in the middle of list first
      if (unusedIndex) {
        // if we go through unused indices and they are not
        // sequential we need to reset gap count
        if (unusedIndex !== index + 1) {
          gapCount = 0;
        }

        index = unusedIndex;
      } else {
        // if unused indices are over just increment index
        // until gap limit will be reached
        index += 1;
      }

      const { privateKey } = this.identities.getIdentityHDKeyByIndex(index, 0);
      const publicKey = privateKey.toPublicKey();

      let identityBuffer;
      try {
        // eslint-disable-next-line no-await-in-loop
        identityBuffer = await this.transport.getIdentityByPublicKeyHash(
          publicKey.hash,
        );
      } catch (e) {
        // if identity is not preset then increment gap count
        // and stop sync if gap limit is reached
        if (e.code === GrpcErrorCodes.NOT_FOUND) {
          gapCount += 1;

          if (gapCount >= this.gapLimit) {
            break;
          }

          // eslint-disable-next-line no-continue
          continue;
        } else {
          throw e;
        }
      }

      // If it's not undefined and not bytes (Uint8Array, which Buffer and
      // Identifier both extend), this method will loop forever.
      // This check prevents this from happening.
      if (!(identityBuffer instanceof Uint8Array)) {
        throw new Error(`Expected identity id to be bytes or undefined, got ${identityBuffer}`);
      }

      // reset gap counter if we got an identity
      // it means gaps are not sequential
      gapCount = 0;

      const identity = Identity.fromBuffer(identityBuffer);

      // eslint-disable-next-line no-await-in-loop
      await this.storage
        .getWalletStore(this.walletId)
        .insertIdentityIdAtIndex(
          identity.getId().toString(),
          index,
        );
    }
  }
}

export default IdentitySyncWorker;