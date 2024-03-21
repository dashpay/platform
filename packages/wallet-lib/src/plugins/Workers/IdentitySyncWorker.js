const { default: loadDpp, Identity } = require('@dashevo/wasm-dpp');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const Worker = require('../Worker');

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
    // Load DPP to make sure Identity and decodeProtocolEntity are available
    await loadDpp();
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

      // If it's not an undefined and not a buffer or Identifier (which inherits Buffer),
      // this method will loop forever.
      // This check prevents this from happening
      if (!Buffer.isBuffer(identityBuffer)) {
        throw new Error(`Expected identity id to be a Buffer or undefined, got ${identityBuffer}`);
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

module.exports = IdentitySyncWorker;
