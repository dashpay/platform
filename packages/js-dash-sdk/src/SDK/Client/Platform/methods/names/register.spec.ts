import { expect } from 'chai';
import { Identifier } from '@dashevo/wasm-dpp';
import { ImportMock } from 'ts-mock-imports';
import generateRandomIdentifier from '@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync';

import cryptoModule from 'crypto';

import register from './register';
import getContract from '../contracts/get';
import { ClientApps } from '../../../ClientApps';

const sha256d = (data: Buffer): Buffer => {
  const sha256 = (bytes: Buffer): Buffer => cryptoModule.createHash('sha256').update(bytes).digest();
  return sha256(sha256(data));
};

// The `domain` schema of DPNS v3: the creator reference finds the creator's own
// preorder by the hash of the normalized label, '.' and the normalized parent
// domain name, and chooses the first preorder of the name holding the same hash
// with its owner's id first; a new name's records.identity is its owner
const dpnsV3DomainSchema = () => ({
  propertyConstraints: {
    recordsIdentityIsOwner: {
      ifThen: [
        { equal: ['$transferredAt', '$createdAt'] },
        { equal: ['records.identity', '$ownerId'] },
      ],
    },
  },
  creatorRefersTo: {
    findBy: {
      $ownerId: '$ownerId',
      nameHash: {
        function: 'sys.hash.sha256d',
        params: ['normalizedLabel', { const: '.' }, 'normalizedParentDomainName'],
      },
    },
    where: { $ownerId: '$ownerId' },
    constraints: {
      minimumAgeBlocks: 1,
      chooseFirst: {
        among: 'nameHash',
        validWhen: {
          ownerNameHash: {
            function: 'sys.hash.sha256d',
            params: ['$ownerId', 'normalizedLabel', { const: '.' }, 'normalizedParentDomainName'],
          },
        },
        by: ['$createdAtBlockHeight'],
      },
    },
    consume: true,
  },
  properties: {
    preorderSalt: {},
  },
});

// The preorder DPNS v3 expects of `identityId` for 'user.dash': the hash of the
// creator and the name, and of the name alone
const dpnsV3Preorder = (identityId) => ({
  ownerNameHash: sha256d(Buffer.concat([identityId.toBuffer(), Buffer.from('user.dash')])),
  nameHash: sha256d(Buffer.from('user.dash')),
});

describe('Platform', () => {
  let randomBytesMock;

  before(() => {
    randomBytesMock = ImportMock.mockFunction(cryptoModule, 'randomBytes', Buffer.alloc(32));
  });
  after(() => {
    randomBytesMock.restore();
  });

  describe('Names', () => {
    describe('#register', () => {
      let platformMock;
      let identityMock;

      beforeEach(async function beforeEach() {
        const contractId = await generateRandomIdentifier();

        platformMock = {
          client: {
            getApps() {
              return new ClientApps({
                dpns: {
                  contractId,
                },
              });
            },
          },
          documents: {
            create: this.sinon.stub(),
            broadcast: this.sinon.stub(),
            // No pending preorder of the name
            get: this.sinon.stub().resolves([]),
          },
          // A DPNS contract declaring no preorder hash: the hash of the salt and the name
          contracts: {
            get: this.sinon.stub().resolves(null),
          },
          initialize: this.sinon.stub(),
        };

        identityMock = {
          getId: this.sinon.stub(),
          getPublicKeyById: this.sinon.stub(),
        };
      });

      it('register top level domain', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);

        await register.call(platformMock, 'Dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.create.getCall(0).args[0]).to.deep.equal('dpns.preorder');
        expect(platformMock.documents.create.getCall(0).args[1]).to.deep.equal(identityMock);
        // sha256d(salt ++ 'Dash'): a top-level name hashes its label as sent
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
          'f6e0b46fa0f455f0b8feabdd25d31e65ea5dbb5f6a9bf34d339099917cf3951b',
        );

        expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
          'dpns.domain',
          identityMock,
          {
            label: 'Dash',
            normalizedLabel: 'dash',
            parentDomainName: '',
            normalizedParentDomainName: '',
            preorderSalt: Buffer.alloc(32),
            records: {
              identity: identityId,
            },
            subdomainRules: {
              allowSubdomains: true,
            },
          },
        ]);
      });

      it('should register second level domain', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.create.getCall(0).args[0]).to.deep.equal('dpns.preorder');
        expect(platformMock.documents.create.getCall(0).args[1]).to.deep.equal(identityMock);
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
          '04a52b75ca842ee9fb14f2cdd27aa0982b9b2cfb2c0e95f640ca3f0c24f1bb9a',
        );

        expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
          'dpns.domain',
          identityMock,
          {
            label: 'User',
            normalizedLabel: 'user',
            parentDomainName: 'dash',
            normalizedParentDomainName: 'dash',
            preorderSalt: Buffer.alloc(32),
            records: {
              identity: identityId,
            },
            subdomainRules: {
              allowSubdomains: false,
            },
          },
        ]);
      });

      it('should hash the parent domain name as sent', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);

        await register.call(platformMock, 'User.DASH', {
          identity: identityId,
        }, identityMock);

        // sha256d(salt ++ 'user' ++ '.' ++ 'DASH'): the platform hashes the parent as sent,
        // not its normalized form
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
          '803f9eba5277949b7dcc60c0b2f0e99fcc4e4c26dfb44b2108c6704328a9c4a7',
        );

        expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
          'dpns.domain',
          identityMock,
          {
            label: 'User',
            normalizedLabel: 'user',
            parentDomainName: 'DASH',
            normalizedParentDomainName: 'dash',
            preorderSalt: Buffer.alloc(32),
            records: {
              identity: identityId,
            },
            subdomainRules: {
              allowSubdomains: false,
            },
          },
        ]);
      });

      it('should hash the params the DPNS contract declares', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });

        await register.call(platformMock, 'User.DASH', {
          identity: identityId,
        }, identityMock);

        // Of the normalized names, without a salt, which the domain no longer sends
        expect(platformMock.documents.create.getCall(0).args[2]).to.deep.equal(
          dpnsV3Preorder(identityId),
        );
        expect(platformMock.documents.create.getCall(1).args[2]).to.not.have.property('preorderSalt');
      });

      it('should hash with the DPNS contract the network stores, not a cached one', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);

        // An app cache warmed with DPNS v2, which declares no preorder hash, on a
        // network that has since stored DPNS v3
        const { contractId } = platformMock.client.getApps().get('dpns');
        const cachedContract = {
          getDocumentSchema: () => ({ properties: { preorderSalt: {} } }),
        };
        const storedContract = {
          getDocumentSchema: dpnsV3DomainSchema,
        };
        const apps = new ClientApps({
          dpns: { contractId, contract: cachedContract },
        });
        platformMock.client.getApps = () => apps;
        platformMock.logger = { debug: () => {}, silly: () => {} };
        platformMock.fetcher = {
          fetchDataContract: async () => ({ getDataContract: () => new Uint8Array() }),
        };
        platformMock.dpp = {
          dataContract: { createFromBuffer: async () => storedContract },
        };
        platformMock.contracts.get = (identifier, options) => getContract.call(
          platformMock,
          identifier,
          options,
        );

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.create.getCall(0).args[2]).to.deep.equal(
          dpnsV3Preorder(identityId),
        );
        // The documents are created against the contract the hash was taken from
        expect(apps.get('dpns').contract).to.equal(storedContract);
      });

      it('should leave the records.identity rule to the platform under DPNS v3', async () => {
        // Fixed ids: `crypto.randomBytes` is mocked to zeros here, so two random
        // identifiers would be the same
        identityMock.getId.returns(Identifier.from(Buffer.alloc(32, 1)));
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });
        const other = Identifier.from(Buffer.alloc(32, 2));

        await register.call(platformMock, 'User.dash', {
          identity: other,
        }, identityMock);

        // The contract's recordsIdentityIsOwner rule is judged by DPP when the domain
        // is created, not copied here: the domain carries the record as given
        expect(platformMock.documents.create.getCall(1).args[0]).to.equal('dpns.domain');
        expect(platformMock.documents.create.getCall(1).args[2].records.identity.equals(other))
          .to.equal(true);
      });

      it('should refuse a creator reference without the chooseFirst hash before preordering', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves({
          getDocumentSchema: () => {
            const schema: any = dpnsV3DomainSchema();
            delete schema.creatorRefersTo.constraints;
            return schema;
          },
        });

        let error;
        try {
          await register.call(platformMock, 'User.dash', {
            identity: identityId,
          }, identityMock);
        } catch (e: any) {
          error = e;
        }

        expect(error.message).to.equal('The DPNS domain\'s creatorRefersTo declares no chooseFirst validWhen function.');
        expect(platformMock.documents.create.called).to.equal(false);
      });

      it('should look for a pending preorder by its owner and name hash under DPNS v3', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.get.getCall(0).args).to.deep.equal([
          'dpns.preorder',
          {
            where: [
              ['$ownerId', '==', identityId],
              ['nameHash', '==', dpnsV3Preorder(identityId).nameHash],
            ],
            limit: 1,
          },
        ]);
      });

      it('should reveal a pending preorder instead of preordering the name again', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });
        platformMock.documents.get.resolves([{ pending: 'preorder' }]);

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        // Only the domain is created and broadcast
        expect(platformMock.documents.create.callCount).to.equal(1);
        expect(platformMock.documents.create.getCall(0).args[0]).to.equal('dpns.domain');
        expect(platformMock.documents.broadcast.callCount).to.equal(1);
      });

      // A pending preorder created at `createdAtMs`, found by a query reporting `nowMs`
      const pendingPreorderAt = (createdAtMs: number, nowMs: number) => ({
        getCreatedAt: () => new Date(createdAtMs),
        getMetadata: () => ({ getTimeMs: () => BigInt(nowMs) }),
      });
      // DPNS v3, whose preorders live an hour
      const dpnsV3Contract = () => ({
        getDocumentSchema: (type: string) => (type === 'preorder'
          ? { ttl: 3600 }
          : dpnsV3DomainSchema()),
      });

      it('should reveal a pending preorder a reveal reaches before it expires', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves(dpnsV3Contract());
        const createdAt = 1_700_000_000_000;
        // Expiring in a minute and a millisecond, past the reveal's margin
        platformMock.documents.get.resolves([
          pendingPreorderAt(createdAt, createdAt + 3_600_000 - 60_001),
        ]);

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.create.callCount).to.equal(1);
        expect(platformMock.documents.create.getCall(0).args[0]).to.equal('dpns.domain');
        expect(platformMock.documents.broadcast.callCount).to.equal(1);
      });

      it('should delete a pending preorder expiring before a reveal lands and preorder again', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves(dpnsV3Contract());
        const createdAt = 1_700_000_000_000;
        // Expired an hour after its creation, but not yet removed by the cleanup
        const expired = pendingPreorderAt(createdAt, createdAt + 3_600_000);
        platformMock.documents.get.resolves([expired]);
        platformMock.documents.create.onCall(0).resolves('preorder document');

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.broadcast.callCount).to.equal(3);
        expect(platformMock.documents.broadcast.getCall(0).args[0]).to.deep.equal({
          delete: [expired],
        });
        expect(platformMock.documents.create.getCall(0).args[0]).to.equal('dpns.preorder');
        expect(platformMock.documents.broadcast.getCall(1).args[0]).to.deep.equal({
          create: ['preorder document'],
        });
        expect(platformMock.documents.create.getCall(1).args[0]).to.equal('dpns.domain');
      });

      it('should fail when the pending preorder query fails', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });
        platformMock.documents.get.rejects(new Error('query failed'));

        let error;
        try {
          await register.call(platformMock, 'User.dash', {
            identity: identityId,
          }, identityMock);
        } catch (e: any) {
          error = e;
        }

        expect(error.message).to.equal('query failed');
        expect(platformMock.documents.create.called).to.equal(false);
        expect(platformMock.documents.broadcast.called).to.equal(false);
      });

      it('should not look for a pending preorder under DPNS v2', async () => {
        const identityId = await generateRandomIdentifier();
        identityMock.getId.returns(identityId);

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        expect(platformMock.documents.get.called).to.equal(false);
        expect(platformMock.documents.create.getCall(0).args[0]).to.equal('dpns.preorder');
      });

      it('should fail if DPNS app is not set up', async () => {
        platformMock.client.getApps = () => new ClientApps({});

        let error;
        try {
          await register.call(platformMock, 'user.dash', {
            identity: await generateRandomIdentifier(),
          }, identityMock);
        } catch (e: any) {
          error = e;
        }

        expect(error.message).to.equal('DPNS is required to register a new name.');
        expect(platformMock.documents.create.called).to.equal(false);
      });
    });
  });
});
