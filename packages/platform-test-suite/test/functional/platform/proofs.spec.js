const Identifier = require('@dashevo/dpp/lib/Identifier');
const { MerkleProof, MerkleTree } = require('js-merkle');
const { executeProof, verifyProof } = require('@dashevo/merk');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const {
  contractId: dpnsContractId,
  ownerId: dpnsOwnerId,
} = require('@dashevo/dpns-contract/lib/systemIds');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const cbor = require('@dashevo/dpp/lib/util/serializer');
const hashFunction = require('../../../lib/proofHashFunction');
const testProofStructure = require('../../../lib/test/testProofStructure');
const parseStoreTreeProof = require('../../../lib/parseStoreTreeProof');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Proofs', () => {
    let blake3;
    let dashClient;
    let contractId;

    before(async () => {
      await hashFunction.init();
      blake3 = hashFunction.hashFunction;

      dashClient = await createClientWithFundedWallet();

      await dashClient.platform.initialize();

      contractId = Identifier.from(dpnsContractId);
    });

    after(() => {
      dashClient.disconnect();
    });

    describe('Store Tree Proofs', () => {
      describe('Data Contract', () => {
        it('should be able to get and verify proof that data contract exists with getIdentity', async () => {
          const dataContractResponseWithProof = await dashClient.getDAPIClient()
            .platform.getDataContract(
              contractId,
              { prove: true },
            );

          const dataContractResponse = await dashClient.getDAPIClient().platform.getDataContract(
            contractId,
          );

          const dataContract = await dashClient.platform.dpp
            .dataContract.createFromBuffer(dataContractResponse.getDataContract());

          const fullProof = dataContractResponseWithProof.getProof();

          testProofStructure(expect, fullProof);

          const dataContractsProofBuffer = fullProof.storeTreeProofs.getDataContractsProof();

          const parsedStoreTreeProof = parseStoreTreeProof(dataContractsProofBuffer);

          expect(parsedStoreTreeProof.values.length).to.be.equal(1);

          const restoredDataContract = await dashClient.platform.dpp
            .dataContract.createFromBuffer(parsedStoreTreeProof.values[0]);

          expect(restoredDataContract.toObject()).to.be.deep.equal(dataContract.toObject());

          const { rootHash: dataContractsLeafRoot } = executeProof(dataContractsProofBuffer);

          const verificationResult = verifyProof(
            dataContractsProofBuffer,
            [contractId],
            dataContractsLeafRoot,
          );

          // We pass one key
          expect(verificationResult.length).to.be.equal(1);

          const recoveredDataContractBuffer = verificationResult[0];
          expect(recoveredDataContractBuffer).to.be.an.instanceof(Uint8Array);

          const recoveredDataContract = await dashClient.platform.dpp
            .dataContract.createFromBuffer(recoveredDataContractBuffer);

          expect(recoveredDataContract.toObject()).to.be.deep.equal(dataContract.toObject());
        });

        it('should be able to verify proof that data contract does not exist', async () => {
          // The same as above, but for an identity id that doesn't exist

          const dataContractId = generateRandomIdentifier();

          const dataContractWithProof = await dashClient.getDAPIClient().platform.getDataContract(
            dataContractId,
            { prove: true },
          );

          const fullProof = dataContractWithProof.proof;

          testProofStructure(expect, fullProof);

          const dataContractsProofBuffer = fullProof.storeTreeProofs.getDataContractsProof();

          const { rootHash: dataContractsLeafRoot } = executeProof(dataContractsProofBuffer);

          const verificationResult = verifyProof(
            dataContractsProofBuffer,
            [dataContractId],
            dataContractsLeafRoot,
          );

          // We pass one key
          expect(verificationResult.length).to.be.equal(1);
          // Data contract doesn't exist, so result is null
          expect(verificationResult[0]).to.be.null();
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
            identityAtKey5 = await dashClient.platform.identities.register(5);
            identityAtKey6 = await dashClient.platform.identities.register(6);
            identityAtKey8 = await dashClient.platform.identities.register(8);

            // await waitForBalanceToChange(walletAccount);

            nonIncludedIdentityPubKeyHash = new PrivateKey().toPublicKey().hash;

            // Public key hashes
            identity6PublicKeyHash = identityAtKey6.getPublicKeyById(0).hash();
            identity8PublicKeyHash = identityAtKey8.getPublicKeyById(0).hash();
          });

          it('should be able to get and verify proof that identity exists with getIdentity', async () => {
            identity = identityAtKey5;

            const identityProof = await dashClient.getDAPIClient().platform.getIdentity(
              identity.getId(),
              { prove: true },
            );

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);

            const identitiesProofBuffer = fullProof.storeTreeProofs.getIdentitiesProof();

            const parsedStoreTreeProof = parseStoreTreeProof(identitiesProofBuffer);

            const parsedIdentity = dashClient.platform.dpp
              .identity.createFromBuffer(parsedStoreTreeProof.values[0]);
            expect(identity.getId()).to.be.deep.equal(parsedIdentity.getId());

            const { rootHash: identityLeafRoot } = executeProof(identitiesProofBuffer);

            const verificationResult = verifyProof(
              identitiesProofBuffer,
              [identity.getId()],
              identityLeafRoot,
            );

            // We pass one key
            expect(verificationResult.length).to.be.equal(1);
            // Identity with id at index 0 doesn't exist
            const recoveredIdentityBuffer = verificationResult[0];
            expect(recoveredIdentityBuffer).to.be.an.instanceof(Uint8Array);

            const recoveredIdentity = dashClient.platform.dpp
              .identity.createFromBuffer(recoveredIdentityBuffer);

            // Deep equal won't work in this case, because identity returned by the register
            const actualIdentity = identity.toObject();
            // Because the actual identity state is before the registration, and the
            // balance wasn't added to it yet
            actualIdentity.balance = recoveredIdentity.toObject().balance;
            expect(recoveredIdentity.toObject()).to.be.deep.equal(actualIdentity);
          });

          it('should be able to verify proof that identity does not exist', async () => {
            // The same as above, but for an identity id that doesn't exist
            const fakeIdentityId = generateRandomIdentifier();

            const identityProof = await dashClient.getDAPIClient().platform.getIdentity(
              fakeIdentityId,
              { prove: true },
            );

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);

            const identitiesProofBuffer = fullProof.storeTreeProofs.getIdentitiesProof();

            // const rootTreeProof = parseRootTreeProof(fullProof.rootTreeProof);
            const parsedStoreTreeProof = parseStoreTreeProof(identitiesProofBuffer);

            const identitiesFromProof = parsedStoreTreeProof.values;

            const valueIds = identitiesFromProof.map((identityValue) => dashClient.platform.dpp
              .identity.createFromBuffer(identityValue).getId().toString('hex'));

            // The proof will contain left and right values to the empty place
            expect(valueIds.indexOf(fakeIdentityId.toString('hex'))).to.be.equal(-1);

            const { rootHash: identityLeafRoot } = executeProof(identitiesProofBuffer);

            const identityIdsToProve = [fakeIdentityId];

            const verificationResult = verifyProof(
              identitiesProofBuffer,
              identityIdsToProve,
              identityLeafRoot,
            );

            // We pass one key
            expect(verificationResult.length).to.be.equal(1);
            // Identity with id at index 0 doesn't exist
            expect(verificationResult[0]).to.be.null();
          });

          it('should be able to verify that multiple identities exist with getIdentitiesByPublicKeyHashes', async () => {
            const publicKeyHashes = [
              identity6PublicKeyHash,
              nonIncludedIdentityPubKeyHash,
              identity8PublicKeyHash,
            ];

            /* Requesting identities by public key hashes and verifying the structure */

            const identityProof = await dashClient.getDAPIClient().platform
              .getIdentitiesByPublicKeyHashes(
                publicKeyHashes,
                { prove: true },
              );

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);

            const identitiesProofBuffer = fullProof.storeTreeProofs.getIdentitiesProof();
            const publicKeyHashesProofBuffer = fullProof.storeTreeProofs
              .getPublicKeyHashesToIdentityIdsProof();

            /* Parsing values from the proof */

            const parsedIdentitiesStoreTreeProof = parseStoreTreeProof(identitiesProofBuffer);

            // Existing identities should be in the identitiesProof, as it also serves
            // as an inclusion proof
            const restoredIdentities = parsedIdentitiesStoreTreeProof.values.map(
              (identityBuffer) => dashClient.platform.dpp.identity.createFromBuffer(
                identityBuffer,
              ),
            );

            /* Figuring out what was found */

            const foundIdentityIds = [];
            const notFoundPublicKeyHashes = [];

            // Scanning through public keys to figure out what identities were found
            for (const publicKeyHash of publicKeyHashes) {
              const foundIdentity = restoredIdentities
                .find(
                  (restoredIdentity) => restoredIdentity.getPublicKeyById(0)
                    .hash().toString('hex') === publicKeyHash.toString('hex'),
                );
              if (foundIdentity) {
                foundIdentityIds.push(foundIdentity.getId());
              } else {
                notFoundPublicKeyHashes.push(publicKeyHash);
              }
            }

            // We expect to find 2 identities out of 3 keys
            expect(foundIdentityIds.length).to.be.equal(2);
            expect(notFoundPublicKeyHashes.length).to.be.equal(1);

            // Note that identities in the proof won't necessary preserve the order in which they
            // were requested. This happens due to the proof structure: sorting values in the
            // proof would result in a different root hash.
            expect(foundIdentityIds.findIndex(
              (identityId) => identityId.toString('hex') === identityAtKey6.getId().toString('hex'),
            )).to.be.greaterThan(-1);
            expect(foundIdentityIds.findIndex(
              (identityId) => identityId.toString('hex') === identityAtKey8.getId().toString('hex'),
            )).to.be.greaterThan(-1);

            expect(notFoundPublicKeyHashes[0]).to.be.deep.equal(nonIncludedIdentityPubKeyHash);

            // Non-existing public key hash should be included into the identityIdsProof,
            // as it serves as a non-inclusion proof for the public keys

            /* Extracting root */

            // While extracting the root isn't specifically useful for this test,
            // it is needed to fit those roots into the root tree later.
            const { rootHash: identityLeafRoot } = executeProof(identitiesProofBuffer);
            const { rootHash: identityIdsLeafRoot } = executeProof(publicKeyHashesProofBuffer);

            /* Inclusion proof */

            // Note that you first has to parse values from the
            // proof and find identity ids you were looking for
            const inclusionVerificationResult = verifyProof(
              identitiesProofBuffer,
              foundIdentityIds,
              identityLeafRoot,
            );

            expect(inclusionVerificationResult.length).to.be.equal(2);

            const firstRecoveredIdentityBuffer = inclusionVerificationResult[0];
            const secondRecoveredIdentityBuffer = inclusionVerificationResult[1];
            expect(firstRecoveredIdentityBuffer).to.be.an.instanceof(Uint8Array);
            expect(secondRecoveredIdentityBuffer).to.be.an.instanceof(Uint8Array);

            const firstRecoveredIdentity = dashClient.platform.dpp
              .identity.createFromBuffer(firstRecoveredIdentityBuffer);

            const secondRecoveredIdentity = dashClient.platform.dpp
              .identity.createFromBuffer(secondRecoveredIdentityBuffer);

            // Deep equal won't work in this case, because identity returned by the register
            const actualIdentityAtKey6 = identityAtKey6.toObject();
            const actualIdentityAtKey8 = identityAtKey8.toObject();
            // Because the actual identity state is before the registration, and the
            // balance wasn't added to it yet
            actualIdentityAtKey6.balance = firstRecoveredIdentity.toObject().balance;
            actualIdentityAtKey8.balance = secondRecoveredIdentity.toObject().balance;

            expect(firstRecoveredIdentity.toObject()).to.be.deep.equal(actualIdentityAtKey6);
            expect(secondRecoveredIdentity.toObject()).to.be.deep.equal(actualIdentityAtKey8);

            /* Non-inclusion proof */

            const nonInclusionVerificationResult = verifyProof(
              publicKeyHashesProofBuffer,
              notFoundPublicKeyHashes,
              identityIdsLeafRoot,
            );

            expect(nonInclusionVerificationResult.length).to.be.equal(1);

            const nonIncludedIdentityId = nonInclusionVerificationResult[0];
            expect(nonIncludedIdentityId).to.be.null();
          });

          it('should be able to verify identityIds with getIdentityIdsByPublicKeyHashes', async () => {
            const publicKeyHashes = [
              identity6PublicKeyHash,
              nonIncludedIdentityPubKeyHash,
              identity8PublicKeyHash,
            ];

            /* Requesting identities by public key hashes and verifying the structure */

            const identityProof = await dashClient.getDAPIClient().platform
              .getIdentityIdsByPublicKeyHashes(
                publicKeyHashes,
                { prove: true },
              );

            const fullProof = identityProof.proof;

            testProofStructure(expect, fullProof);

            const publicKeyHashesProofBuffer = fullProof.storeTreeProofs
              .getPublicKeyHashesToIdentityIdsProof();

            /* Extracting root */

            const {
              rootHash: publicKeyHashesToIdentityIdsLeafRoot,
            } = executeProof(publicKeyHashesProofBuffer);

            /* Verifying proof */

            // Note that you first has to parse values from the
            // proof and find identity ids you were looking for
            const verificationResult = verifyProof(
              publicKeyHashesProofBuffer,
              publicKeyHashes,
              publicKeyHashesToIdentityIdsLeafRoot,
            );

            expect(verificationResult.length).to.be.equal(3);

            const firstIdentityId = verificationResult[0];
            const secondIdentityId = verificationResult[1];
            const thirdIdentityId = verificationResult[2];

            expect(firstIdentityId).to.be.an.instanceof(Uint8Array);
            // In the verifyProof call, non existing key is passed as a second element
            // and verifyProof returns values sorted in the same way as they were
            // passed to the function
            expect(secondIdentityId).to.be.null();
            expect(thirdIdentityId).to.be.an.instanceof(Uint8Array);

            expect(firstIdentityId).to.be.deep.equal(cbor.encode([identityAtKey6.getId()]));
            expect(thirdIdentityId).to.be.deep.equal(cbor.encode([identityAtKey8.getId()]));
          });
        });
      });
    });

    describe('Root Tree Proof', () => {
      it('should be correct for all endpoints', async () => {
        // This test requests all endpoints instead of having multiple test for each endpoint
        // on purpose.
        //
        // The reason being is that when verifying merkle proof, you usually need some value to
        // compare it to, and platform doesn't provide one. There are two ways to verify that
        // the root tree proof is working: either by knowing its root in advance, or by
        // verifying it's signature that is also included in the response.
        // Verifying signature requires verifying the header chain, which is not
        // currently implemented in the JS SDK (Although it is implemented in Java and iOS SDK).
        // So we left with only one option: to know the proof in advance.
        // Platform doesn't give it directly, but we can reconstruct it from
        // store tree leaves. This if fine in this case because this test doesn't test
        // store tree proofs (every endpoint has its own separate store tree proof test).
        // By making requests to all endpoints we can recover all leaves hashes, and construct
        // the original root tree from it. Then we can get the root from that tree and use it
        // as a reference root when verifying the root tree proof.

        const dapiClient = await dashClient.getDAPIClient();
        const identityId = Identifier.from(dpnsOwnerId);
        const identity = await dashClient.platform.identities.get(identityId);

        const [
          identityResponse,
          keysResponse,
          contractsResponse,
          documentsResponse,
          identitiesByPublicKeyHashesResponse,
        ] = await Promise.all([
          dapiClient.platform.getIdentity(identityId, { prove: true }),
          dapiClient.platform.getIdentityIdsByPublicKeyHashes(
            [identity.getPublicKeyById(0).getData()],
            { prove: true },
          ),
          dapiClient.platform.getDataContract(contractId, { prove: true }),
          dapiClient.platform.getDocuments(
            contractId,
            'preorder',
            {
              where: [['$id', '==', identityId]],
              prove: true,
            },
          ),
          dapiClient.platform.getIdentitiesByPublicKeyHashes(
            [identity.getPublicKeyById(0).getData()],
            { prove: true },
          ),
        ]);

        const identityProof = MerkleProof.fromBuffer(
          identityResponse.proof.rootTreeProof,
          blake3,
        );
        const contractsProof = MerkleProof.fromBuffer(
          contractsResponse.proof.rootTreeProof,
          blake3,
        );
        const documentsProof = MerkleProof.fromBuffer(
          documentsResponse.proof.rootTreeProof,
          blake3,
        );
        const keysProof = MerkleProof.fromBuffer(
          keysResponse.proof.rootTreeProof,
          blake3,
        );
        const identitiesByPublicKeyHashesProof = MerkleProof.fromBuffer(
          identitiesByPublicKeyHashesResponse.proof.rootTreeProof,
          blake3,
        );

        const { rootHash: identityLeaf } = executeProof(
          identityResponse.proof.storeTreeProofs.getIdentitiesProof(),
        );
        const { rootHash: publicKeysLeaf } = executeProof(
          keysResponse.proof.storeTreeProofs.getPublicKeyHashesToIdentityIdsProof(),
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
          publicKeysLeaf,
          contractsLeaf,
          documentsLeaf,
          documentsProof.getProofHashes()[0],
        ];

        const reconstructedTree = new MerkleTree(reconstructedLeaves, blake3);
        const treeLayers = reconstructedTree.getHexLayers();
        const reconstructedAppHash = Buffer.from(reconstructedTree.getRoot()).toString('hex');

        const identityProofRoot = Buffer.from(identityProof.calculateRoot([1], [identityLeaf], 6)).toString('hex');
        const keysProofRoot = Buffer.from(keysProof.calculateRoot([2], [publicKeysLeaf], 6)).toString('hex');
        const contractsProofRoot = Buffer.from(contractsProof.calculateRoot([3], [contractsLeaf], 6)).toString('hex');
        const documentsProofRoot = Buffer.from(documentsProof.calculateRoot([4], [documentsLeaf], 6)).toString('hex');
        const identitiesIdsProofRoot = Buffer.from(identitiesByPublicKeyHashesProof.calculateRoot([1, 2], [identityLeaf, publicKeysLeaf], 6)).toString('hex');

        expect(identityProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][0],
          treeLayers[1][1],
          treeLayers[1][2],
        ]);

        expect(keysProof.getHexProofHashes()).to.be.deep.equal([
          treeLayers[0][3],
          treeLayers[1][0],
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
        expect(keysProofRoot).to.be.equal(reconstructedAppHash);
        expect(contractsProofRoot).to.be.equal(reconstructedAppHash);
        expect(documentsProofRoot).to.be.equal(reconstructedAppHash);
        expect(identitiesIdsProofRoot).to.be.equal(reconstructedAppHash);
      });
    });
  });
});
