const Identifier = require('@dashevo/dpp/lib/Identifier');
const Worker = require('../Worker');

const logger = require('../../logger');

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
      gapLimit: 10,
      dependencies: [
        'storage',
        'transport',
        'walletId',
        'getIdentityHDKeyByIndex',
      ],
      ...options,
    });
  }

  async execute() {
    const indexedIds = await this.storage.getIndexedIdentityIds(this.walletId);

    // Add gaps to empty indices
    const unusedIndices = [];
    indexedIds.forEach((id, index) => {
      if (!id) {
        return;
      }

      unusedIndices.push(index);
    });

    logger.silly('IdentitySyncWorker - sync start');

    let gapCount = 0;
    let unusedIndex;
    let index = -1;
    while (gapCount < this.gapLimit) {
      unusedIndex = unusedIndices.shift();

      // check unused indices in the middle of list first
      if (unusedIndex) {
        // if we go through unused indices and thay are not
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

      const { privateKey } = this.getIdentityHDKeyByIndex(index, 0);
      const publicKey = privateKey.toPublicKey();

      // eslint-disable-next-line no-await-in-loop
      const [fetchedId] = await this.transport.getIdentityIdsByPublicKeyHash([publicKey.hash]);

      // if identity id is not preset then increment gap count
      // and stop sync if gap limit is reached
      if (fetchedId === null) {
        gapCount += 1;

        logger.silly(`IdentitySyncWorker - gap at index ${index}`);

        if (gapCount >= this.gapLimit) {
          logger.silly('IdentitySyncWorker - gap limit is reached');

          break;
        }

        // eslint-disable-next-line no-continue
        continue;
      }

      // If it's not a null and not a buffer or Identifier (which inherits Buffer),
      // this method will loop forever.
      // This check prevents this from happening
      if (!Buffer.isBuffer(fetchedId)) {
        throw new Error(`Expected identity id to be a Buffer or null, got ${fetchedId}`);
      }

      // reset gap counter if we got an identity
      // it means gaps are not sequential
      gapCount = 0;

      logger.silly(`IdentitySyncWorker - got ${fetchedId} at ${index}`);

      // eslint-disable-next-line no-await-in-loop
      await this.storage.insertIdentityIdAtIndex(
        this.walletId,
        Identifier.from(fetchedId).toString(),
        index,
      );
    }

    logger.silly('IdentitySyncWorker - sync finished');
  }
}

module.exports = IdentitySyncWorker;
