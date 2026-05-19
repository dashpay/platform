import Dash from 'dash';
import { MerkleProof, MerkleTree } from 'js-merkle';
import {
  contractId as dpnsContractId,
  ownerId as dpnsOwnerId,
} from '@dashevo/dpns-contract/lib/systemIds.js';

import generateRandomIdentifier from '../../../lib/test/utils/generateRandomIdentifier.js';
import testProofStructure from '../../../lib/test/testProofStructure.js';
// import parseStoreTreeProof from '../../../lib/parseStoreTreeProof.js';
import createClientWithFundedWallet from '../../../lib/test/createClientWithFundedWallet.js';

const {
  Core: {
    PrivateKey,
  },
  PlatformProtocol: {
    Identifier,
  },
} = Dash;

function executeProof() {

}

// TODO: Fix test to be running in Browser

describe('Platform', () => {
  describe('Proofs', () => {
    let blake3;
    let dashClient;
    let contractId;

    before(async () => {
      dashClient = await createClientWithFundedWallet(1000000);

      await dashClient.platform.initialize();

      contractId = Identifier.from(dpnsContractId);
    });

    after(() => {
      if (dashClient) {
        dashClient.disconnect();
      }
    });

    describe('Merkle Proofs', () => {
      describe('Data Contract', () => {
        it('should be able to get and verify proof that data contract exists with getIdentity', async () => {
          const dataContractResponseWithProof = await dashClient.getDAPIClient()
            .platform.getDataContract(contractId, { prove: true });

          const fullProof = dataContractResponseWithProof.getProof();

          testProofStructure(expect, fullProof);
        });

        // TODO(rs-drive-abci): restore.
        it.skip('should be able to verify proof that data contract does not exist', async () => {
          // The same as above, but for an identity id that doesn't exist

          const dataContractId = await generateRandomIdentifier();

          const dataContractWithProof = await dashClient.getDAPIClient()
            .platform.getDataContract(dataContractId, { prove: true });

          const fullProof = dataContractWithProof.proof;

          testProofStructure(expect, fullProof);
        });
      });

      describe('Identities', () => {
        describe('Proofs', () => {
          let identity;
          let identityAtKey5;
          let identityAtKey6;
          let identityAtKey8;
          let nonIncludedIdentityPubKeyHash;
          let identity6PublicKeyHash;
          let identity8PublicKeyHash;

          before(async () => {
            identityAtKey5 = await dashClient.platform.identities.register(230000);

            identityAtKey6 = await dashClient.platform.identities.register(230000);

            identityAtKey8 = await dashClient.platform.identities.register(230000);

            nonIncludedIdentityPubKeyHash = new PrivateKey().toPublicKey().hash;

            // Public key hashes
            identity6PublicKeyHash = identityAtKey6.getPublicKeyById(0).hash();
            identity8PublicKeyHash = identityAtKey8.getPublicKeyById(0).hash();
          });

          it('should be able to get and verify proof that identity exists with getIdentity', async () => {
            identity = identityAtKey5;

            const identityProof = await dashClient.getDAPIClient()
              .platform.getIdentity(identity.getId(), { prove: true });

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);
          });

          it('should be able to verify proof that identity does not exist', async () => {
            // The same as above, but for an identity id that doesn't exist
            const fakeIdentityId = await generateRandomIdentifier();

            const identityProof = await dashClient.getDAPIClient()
              .platform.getIdentity(fakeIdentityId, { prove: true });

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);
          });

          // eslint-disable-next max-len
          it.skip('should be able to verify that multiple identities exist with getIdentitiesByPublicKeyHashes', async () => {
            const publicKeyHashes = [
              identity6PublicKeyHash,
              nonIncludedIdentityPubKeyHash,
              identity8PublicKeyHash,
            ];

            /* Requesting identities by public key hashes and verifying the structure */

            const identityProof = await dashClient.getDAPIClient().platform
              .getIdentitiesByPublicKeyHashes(publicKeyHashes, { prove: true });

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);
          });
        });
      });
    });

    describe.skip('Root Tree Proof', () => {
      it('should be correct for all endpoints', async () => {
        // This test requests all endpoints instead of having multiple test for each endpoint
        // on purpose.

        const dapiClient = await dashClient.getDAPIClient();
        const identityId = Identifier.from(dpnsOwnerId);
        const identity = await dashClient.platform.identities.get(identityId);

        const [
          identityResponse,
          contractsResponse,
          documentsResponse,
          identitiesByPublicKeyHashesResponse,
        ] = await Promise.all([
          dapiClient.platform.getIdentity(identityId, { prove: true }),
          dapiClient.platform.getDataContract(contractId, { prove: true }),
          dapiClient.platform.getDocuments(contractId, 'preorder', {
            where: [['$id', '==', identityId]],
            prove: true,
          }),
          dapiClient.platform.getIdentitiesByPublicKeyHashes([
            identity.getPublicKeyById(0).getData()], { prove: true }),
        ]);

        const identityProof = MerkleProof.fromBuffer(
          identityResponse.proof.rootTreeProof,
          'fake_blake3',
        );
        const contractsProof = MerkleProof.fromBuffer(
          contractsResponse.proof.rootTreeProof,
          'fake_blake3',
        );
        const documentsProof = MerkleProof.fromBuffer(
          documentsResponse.proof.rootTreeProof,
          'fake_blake3',
        );
        const identitiesByPublicKeyHashesProof = MerkleProof
          .fromBuffer(identitiesByPublicKeyHashesResponse.proof.rootTreeProof, 'fake_blake3');

        const { rootHash: identityLeaf } = executeProof(
          identityResponse.proof.storeTreeProofs.getIdentitiesProof(),
        );

        const { rootHash: contractsLeaf } = executeProof(
          contractsResponse.proof.storeTreeProofs.getDataContractsProof(),
        );
        const { rootHash: documentsLeaf } = executeProof(
          documentsResponse.proof.storeTreeProofs.getDocumentsProof(),
        );

        const reconstructedLeaves = [
          identityProof.getProofHashes()[0],
          identityLeaf,
          contractsLeaf,
          documentsLeaf,
          documentsProof.getProofHashes()[0],
        ];

        const reconstructedTree = new MerkleTree(reconstructedLeaves, blake3);
        const treeLayers = reconstructedTree.getHexLayers();
        const reconstructedAppHash = Buffer.from(reconstructedTree.getRoot()).toString('hex');

        const identityProofRoot = Buffer.from(identityProof.calculateRoot([1], [identityLeaf], 6)).toString('hex');
        const contractsProofRoot = Buffer.from(contractsProof.calculateRoot([3], [contractsLeaf], 6)).toString('hex');
        const documentsProofRoot = Buffer.from(documentsProof.calculateRoot([4], [documentsLeaf], 6)).toString('hex');

        expect(identityProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][0],
          treeLayers[1][1],
          treeLayers[1][2],
        ]);

        expect(contractsProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][2],
          treeLayers[1][0],
          treeLayers[1][2],
        ]);

        expect(documentsProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][5],
          treeLayers[2][0],
        ]);

        expect(identitiesByPublicKeyHashesProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][0],
          treeLayers[0][3],
          treeLayers[1][2],
        ]);

        expect(identityProofRoot).to.be.equal(reconstructedAppHash);
        expect(contractsProofRoot).to.be.equal(reconstructedAppHash);
        expect(documentsProofRoot).to.be.equal(reconstructedAppHash);
      });
    });
  });
});
