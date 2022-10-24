const Dash = require('dash');
const crypto = require('crypto');

const { MerkleProof } = require('js-merkle');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

const parseStoreTreeProof = require('../../../lib/parseStoreTreeProof');
const hashFunction = require('../../../lib/proofHashFunction');

describe.skip('Platform', () => {
  describe('waitForStateTransitionResult', () => {
    let dpp;
    let client;
    let blake3;

    before(async () => {
      dpp = new Dash.PlatformProtocol();
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
      } = await client.platform.identities.utils
        .createAssetLockTransaction(10000);

      // Broadcast Asset Lock transaction
      await account.broadcastTransaction(assetLockTransaction);
      const assetLockProof = await client.platform.identities.utils
        .createAssetLockProof(assetLockTransaction, assetLockOutputIndex);

      const {
        identity, identityCreateTransition,
      } = await client.platform.identities.utils
        .createIdentityCreateTransition(assetLockProof, assetLockPrivateKey);

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

      function executeProof() {

      }

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
