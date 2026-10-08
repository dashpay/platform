import { expect } from 'chai';
import { ImportMock } from 'ts-mock-imports';
import generateRandomIdentifier from '@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync';

import cryptoModule from 'crypto';

import register from './register';
import { ClientApps } from '../../../ClientApps';

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
        expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
          'dad5808c709bec62b65e607b38846d4fe4080b251cb917a2221c81fb7c3b5f5e',
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

      it('should fail if DPNS app have no contract set up', async () => {
        delete platformMock.client.getApps().get('dpns').contractId;

        try {
          await register.call(platformMock, 'user.dash', {
            identity: await generateRandomIdentifier(),
          }, identityMock);
        } catch (e: any) {
          expect(e.message).to.equal('DPNS is required to register a new name.');
        }
      });
    });
  });
});
