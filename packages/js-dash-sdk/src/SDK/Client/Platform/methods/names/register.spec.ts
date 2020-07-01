import { expect } from 'chai';
import { ImportMock } from 'ts-mock-imports';

import entropyModule from '@dashevo/dpp/lib/util/entropy';

ImportMock.mockFunction(entropyModule, 'generate', 'someEntropy');

import register from './register';

describe('Platform', () => {
    describe('Names', () => {
        describe('#register', () => {
            let platformMock;
            let identityMock;

            beforeEach(async function beforeEach() {
                platformMock = {
                    apps: {
                      dpns: {
                          contractId: 'someDPNSContractId',
                      },
                    },
                    documents: {
                        create: this.sinon.stub(),
                        broadcast: this.sinon.stub(),
                    },
                };

                identityMock = {
                    getId: this.sinon.stub(),
                    getPublicKeyById: this.sinon.stub(),
                };
            });

            it('register top level domain', async () => {
                identityMock.getId.returns('someIdentityId');

                await register.call(platformMock, 'Dash', identityMock);

                expect(identityMock.getId.callCount).to.equal(1);
                expect(platformMock.documents.create.getCall(0).args).to.have.deep.members([
                    'dpns.preorder',
                    identityMock,
                    {
                        "saltedDomainHash": "5620d033a9faaa2633afd855570b28b11190a5f2d16634021eb0acf8cee7d402b756",
                    },
                ]);

                expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
                    'dpns.domain',
                    identityMock,
                    {
                        'label': 'Dash',
                        'nameHash': '562060f0833932a21446ada9b0bb71ac8e8b40e2618f99f44204d66815f6bdf258cc',
                        'normalizedLabel': 'dash',
                        'normalizedParentDomainName': '',
                        'preorderSalt': 'someEntropy',
                        'records': {
                            'dashIdentity': 'someIdentityId',
                        }
                    }
                ]);
            });

            it('should register second level domain', async () => {
                identityMock.getId.returns('someIdentityId');

                await register.call(platformMock, 'User.dash', identityMock);

                expect(identityMock.getId.callCount).to.equal(1);
                expect(platformMock.documents.create.getCall(0).args).to.have.deep.members([
                    'dpns.preorder',
                    identityMock,
                    {
                        "saltedDomainHash": "5620ca5eed7648dcb77804e80c17753c23b1b77d43ca9ead90dee01c9c913ca4f13e",
                    },
                ]);

                expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
                    'dpns.domain',
                    identityMock,
                    {
                        'label': 'User',
                        'nameHash': '5620b5f42fb635a08cc0f441bbc6ef5f3bdeed2877692feffd9945bde3abf8b4141f',
                        'normalizedLabel': 'user',
                        'normalizedParentDomainName': 'dash',
                        'preorderSalt': 'someEntropy',
                        'records': {
                            'dashIdentity': 'someIdentityId',
                        }
                    }
                ]);
            });

            it('should fail if DPNS app have no contract set up', async () => {
                delete platformMock.apps.dpns.contractId;

                try {
                    await register.call(platformMock, 'user.dash', identityMock);
                } catch (e) {
                    expect(e.message).to.equal('DPNS is required to register a new name.');
                }
            });
        });
    });
});
