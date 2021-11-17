import { expect } from 'chai';
import { ImportMock } from 'ts-mock-imports';
import generateRandomIdentifier from "@dashevo/dpp/lib/test/utils/generateRandomIdentifier"

import cryptoModule from 'crypto';

import register from './register';
import {ClientApps} from "../../../ClientApps";

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
                platformMock = {
                    client: {
                        getApps() {
                            return new ClientApps({
                                dpns: {
                                    contractId: generateRandomIdentifier(),
                                }
                            });
                        }
                    },
                    documents: {
                        create: this.sinon.stub(),
                        broadcast: this.sinon.stub(),
                    },
                    initialize:  this.sinon.stub(),
                };

                identityMock = {
                    getId: this.sinon.stub(),
                    getPublicKeyById: this.sinon.stub(),
                };
            });

            it('register top level domain', async () => {
                const identityId = generateRandomIdentifier();
                identityMock.getId.returns(identityId);

                await register.call(platformMock, 'Dash', {
                    dashUniqueIdentityId: identityId,
                }, identityMock);

                expect(platformMock.documents.create.getCall(0).args[0]).to.deep.equal('dpns.preorder');
                expect(platformMock.documents.create.getCall(0).args[1]).to.deep.equal(identityMock);
                expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
                    '0a7c796d4d76f0d536c4a6efdffa0647aac6e74239251a5c1c13dc5d4d414a98',
                );

                expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
                    'dpns.domain',
                    identityMock,
                    {
                        'label': 'Dash',
                        'normalizedLabel': 'dash',
                        'normalizedParentDomainName': '',
                        'preorderSalt': Buffer.alloc(32),
                        'records': {
                            'dashUniqueIdentityId': identityId,
                        },
                        'subdomainRules': {
                            'allowSubdomains': true,
                        },
                    }
                ]);
            });

            it('should register second level domain', async () => {
                const identityId = generateRandomIdentifier();
                identityMock.getId.returns(identityId);

                await register.call(platformMock, 'User.dash', {
                    dashAliasIdentityId: identityId,
                }, identityMock);

                expect(platformMock.documents.create.getCall(0).args[0]).to.deep.equal('dpns.preorder');
                expect(platformMock.documents.create.getCall(0).args[1]).to.deep.equal(identityMock);
                expect(platformMock.documents.create.getCall(0).args[2].saltedDomainHash.toString('hex')).to.deep.equal(
                    '23ded03e8006422aa5cd19003713e60c87902590084b3f1dbea26045a93c517d',
                );

                expect(platformMock.documents.create.getCall(1).args).to.have.deep.members([
                    'dpns.domain',
                    identityMock,
                    {
                        'label': 'User',
                        'normalizedLabel': 'user',
                        'normalizedParentDomainName': 'dash',
                        'preorderSalt': Buffer.alloc(32),
                        'records': {
                            'dashAliasIdentityId': identityId,
                        },
                        'subdomainRules': {
                            'allowSubdomains': false,
                        },
                    }
                ]);
            });

            it('should fail if DPNS app have no contract set up', async () => {
                delete platformMock.client.getApps().get('dpns').contractId;

                try {
                    await register.call(platformMock, 'user.dash', {
                        dashUniqueIdentityId: generateRandomIdentifier(),
                    }, identityMock);
                } catch (e) {
                    expect(e.message).to.equal('DPNS is required to register a new name.');
                }
            });
        });
    });
});
