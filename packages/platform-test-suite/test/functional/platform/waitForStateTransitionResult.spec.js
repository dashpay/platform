const DashPlatformProtocol = require('@dashevo/dpp');
const crypto = require('crypto');

const { default: createAssetLockProof } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/createAssetLockProof');
const { default: createIdentityCreateTransition } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/createIdentityCreateTransition');
const { default: createAssetLockTransaction } = require('dash/build/src/SDK/Client/Platform/createAssetLockTransaction');

const { MerkleProof } = require('js-merkle');
const { executeProof } = require('@dashevo/merk');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

const parseStoreTreeProof = require('../../../lib/parseStoreTreeProof');
const hashFunction = require('../../../lib/proofHashFunction');

describe('Platform', () => {
  describe('waitForStateTransitionResult', () => {
    let dpp;
    let client;
    let blake3;

    before(async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      await hashFunction.init();
      blake3 = hashFunction.hashFunction;

      client = await createClientWithFundedWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should return a correct proof with a value', async () => {
      const account = await client.getWalletAccount();

      const {
        transaction: assetLockTransaction,
        privateKey: assetLockPrivateKey,
        outputIndex: assetLockOutputIndex,
      } = await createAssetLockTransaction({ client }, 10000);

      // Broadcast Asset Lock transaction
      await account.broadcastTransaction(assetLockTransaction);
      const assetLockProof = await createAssetLockProof(
        client.platform, assetLockTransaction, assetLockOutputIndex,
      );

      const {
        identity, identityCreateTransition,
      } = await createIdentityCreateTransition(
        client.platform, assetLockProof, assetLockPrivateKey,
      );

      const hash = crypto.createHash('sha256')
        .update(identityCreateTransition.toBuffer())
        .digest();

      await client.platform.broadcastStateTransition(
        identityCreateTransition,
      );

      /*  Waiting for the result and parse the proof  */

      const result = await client.getDAPIClient()
        .platform
        .waitForStateTransitionResult(hash, { prove: true });

      const { rootTreeProof } = result.proof;
      const identitiesProofBuffer = result.proof.storeTreeProofs.identitiesProof;

      const parsedStoreTreeProof = parseStoreTreeProof(identitiesProofBuffer);

      const { rootHash: identityLeafRoot } = executeProof(identitiesProofBuffer);

      const identityProof = MerkleProof.fromBuffer(
        rootTreeProof, blake3,
      );
      Buffer
        .from(
          identityProof.calculateRoot([1], [identityLeafRoot], 6),
        )
        .toString('hex');
      const parsedIdentity = client.platform.dpp
        .identity.createFromBuffer(parsedStoreTreeProof.values[0]);

      expect(identity.getId()).to.be.deep.equal(parsedIdentity.getId());
    });
  });
});
