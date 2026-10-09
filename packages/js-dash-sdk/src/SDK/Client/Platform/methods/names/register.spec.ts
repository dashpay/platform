import { expect } from 'chai';
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

// The `domain` schema of DPNS v3: the salt reveals the hash of the writer's id,
// the salt, the normalized label, '.' and the parent domain name, and a new
// name's records.identity is its owner
const dpnsV3DomainSchema = () => ({
  propertyConstraints: {
    recordsIdentityIsOwner: {
      ifThen: [
        { equal: ['$transferredAt', '$createdAt'] },
        { equal: ['records.identity', '$ownerId'] },
      ],
    },
  },
  properties: {
    preorderSalt: {
      refersTo: {
        findBy: {
          saltedDomainHash: {
            function: 'sys.hash.sha256d',
            params: ['$ownerId', 'preorderSalt', 'normalizedLabel', { const: '.' }, 'parentDomainName'],
          },
        },
      },
    },
  },
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

        await register.call(platformMock, 'User.dash', {
          identity: identityId,
        }, identityMock);

        // sha256d(owner id ++ salt ++ 'user' ++ '.' ++ 'dash')
        const expected = sha256d(Buffer.concat([
          identityId.toBuffer(),
          Buffer.alloc(32),
          Buffer.from('user.dash'),
        ]));
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.equal(
          expected.toString('hex'),
        );
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

        // sha256d(owner id ++ salt ++ 'user' ++ '.' ++ 'dash')
        const expected = sha256d(Buffer.concat([
          identityId.toBuffer(),
          Buffer.alloc(32),
          Buffer.from('user.dash'),
        ]));
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.equal(
          expected.toString('hex'),
        );
        // The documents are created against the contract the hash was taken from
        expect(apps.get('dpns').contract).to.equal(storedContract);
      });

      it('should refuse records.identity other than the registering identity under DPNS v3', async () => {
        identityMock.getId.returns(await generateRandomIdentifier());
        platformMock.contracts.get.resolves({
          getDocumentSchema: dpnsV3DomainSchema,
        });

        let error;
        try {
          await register.call(platformMock, 'User.dash', {
            identity: await generateRandomIdentifier(),
          }, identityMock);
        } catch (e: any) {
          error = e;
        }

        expect(error.message).to.equal('records.identity must be the identity registering the name.');
        expect(platformMock.documents.create.called).to.equal(false);
        expect(platformMock.documents.broadcast.called).to.equal(false);
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
