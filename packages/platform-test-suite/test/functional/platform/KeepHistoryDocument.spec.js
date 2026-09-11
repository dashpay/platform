const Dash = require('dash');
const { expect } = require('chai');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');
const { getEvoSdkForNetwork } = require('../../../lib/test/createPlatformProofVerifier');

const {
  Errors: {
    StateTransitionBroadcastError,
  },
} = Dash;

/**
 * The identity's first key and a signer holding its private key, in the shape
 * the Evo SDK's write methods expect. The legacy client owns the wallet, so the
 * key has to be lifted out of it.
 *
 * @param {Object} evo
 * @param {Object} evoSdk
 * @param {Object} client
 * @param {Object} identity
 * @returns {Promise<{ identityKey: Object, signer: Object }>}
 */
async function evoSignerFor(evo, evoSdk, client, identity) {
  const account = await client.getWalletAccount();
  const { privateKey } = account.identities.getIdentityHDKeyById(
    identity.getId().toString(),
    0,
  );

  const signer = new evo.IdentitySigner();
  signer.addKeyFromWif(privateKey.toWIF());

  const fetched = await evoSdk.identities.fetch(identity.getId().toString());
  const identityKey = fetched.getPublicKeyById(0);

  return { identityKey, signer };
}

describe('Platform', () => {
  describe('KeepHistoryDocument', function main() {
    this.timeout(900000);

    let client;
    let evo;
    let evoSdk;
    let identity;
    let dataContract;
    let note;

    /**
     * The revision count and lifecycle metadata Platform authenticates for the
     * note, read through the Evo SDK's history v1 query.
     */
    const readHistory = async () => {
      const result = await evoSdk.documents.history({
        dataContractId: dataContract.getId().toString(),
        documentTypeName: 'note',
        documentId: note.getId().toString(),
        startAtMs: 0,
        limit: 10,
      });

      return result.lifecycle;
    };

    before(async () => {
      client = await createClientWithFundedWallet(400000000); // 4 Dash

      identity = await client.platform.identities.register(200000000);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      ({ evo, sdk: evoSdk } = await getEvoSdkForNetwork(process.env.NETWORK));
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should register a contract whose notes keep history and may be erased', async () => {
      dataContract = await client.platform.contracts.create({
        note: {
          type: 'object',
          documentsKeepHistory: true,
          documentsMutable: true,
          canBeDeleted: true,
          canBeErased: true,
          properties: {
            message: {
              type: 'string',
              maxLength: 256,
              position: 0,
            },
          },
          required: ['message'],
          additionalProperties: false,
        },
      }, identity);

      await client.platform.contracts.publish(dataContract, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      client.getApps().set('notes', {
        contractId: dataContract.getId(),
        contract: dataContract,
      });

      const fetched = await client.platform.contracts.get(dataContract.getId());
      expect(fetched.toObject().documentSchemas.note.canBeErased).to.be.true();
    });

    it('should keep every revision of a note that is edited', async () => {
      note = await client.platform.documents.create(
        'notes.note',
        identity,
        { message: 'first' },
      );

      await client.platform.documents.broadcast({ create: [note] }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      for (const message of ['second', 'third']) {
        // The SDK signs `revision + 1` from the document it is handed but never
        // bumps that local copy, so every replace starts from a fresh fetch.
        const [stored] = await client.platform.documents.get(
          'notes.note',
          { where: [['$id', '==', note.getId()]] },
        );
        stored.set('message', message);
        await client.platform.documents.broadcast({ replace: [stored] }, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();
      }

      const lifecycle = await readHistory();
      expect(lifecycle.state).to.equal('ACTIVE');
      expect(lifecycle.remainingRevisions).to.equal(3n);
      expect(lifecycle.deletedAtMs).to.equal(0n);
    });

    it('should hide a deleted note while keeping its revisions readable', async () => {
      await client.platform.documents.broadcast({ delete: [note] }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const [found] = await client.platform.documents.get(
        'notes.note',
        { where: [['$id', '==', note.getId()]] },
      );
      expect(found).to.be.undefined();

      const lifecycle = await readHistory();
      expect(lifecycle.state).to.equal('DELETED');
      expect(
        lifecycle.remainingRevisions,
        'a delete removes no revision',
      ).to.equal(3n);
      expect(lifecycle.deletedAtMs > 0n).to.be.true();
      expect(lifecycle.erasingStartedAtMs).to.equal(0n);
    });

    it('should refuse to delete the same note twice', async () => {
      let broadcastError;

      try {
        await client.platform.documents.broadcast({ delete: [note] }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      // DocumentNotFoundError: a document nothing can read is not there to be
      // deleted again, and a delete never escalates into removing a revision.
      expect(broadcastError.code).to.equal(40101);
    });

    it('should erase the retained revisions of a deleted note', async () => {
      const { identityKey, signer } = await evoSignerFor(evo, evoSdk, client, identity);

      await evoSdk.documents.erase({
        document: {
          id: note.getId().toString(),
          ownerId: identity.getId().toString(),
          dataContractId: dataContract.getId().toString(),
          documentTypeName: 'note',
        },
        identityKey,
        signer,
      });

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const lifecycle = await readHistory();
      expect(lifecycle.state).to.equal('ABSENT');
      expect(lifecycle.remainingRevisions).to.equal(0n);
    });

    it('should refuse to erase a note that has already been erased', async () => {
      const { identityKey, signer } = await evoSignerFor(evo, evoSdk, client, identity);

      let eraseError;

      try {
        await evoSdk.documents.erase({
          document: {
            id: note.getId().toString(),
            ownerId: identity.getId().toString(),
            dataContractId: dataContract.getId().toString(),
            documentTypeName: 'note',
          },
          identityKey,
          signer,
        });
      } catch (e) {
        eraseError = e;
      }

      expect(eraseError, 'an id that holds nothing is not erasable').to.exist();
    });
  });
});
