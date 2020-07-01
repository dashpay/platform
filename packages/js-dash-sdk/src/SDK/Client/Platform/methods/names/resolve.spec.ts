import { expect } from 'chai';

import resolve from './resolve';

describe('Platform', () => {
    describe('Names', () => {
        describe('#resolve', () => {
            let platformMock;
            let parentDomainDocument;
            let childDomainDocument;

            beforeEach(async function beforeEach() {
                parentDomainDocument = { label: 'parent' };
                childDomainDocument = { label: 'child.parent' };

                platformMock = {
                    documents: {
                        get: this.sinon.stub(),
                    },
                };
            });

            it('should resolve domain by it\'s name', async () => {
                platformMock.documents.get.resolves([parentDomainDocument]);

                const receivedDocument = await resolve.call(platformMock, 'parent');

                expect(platformMock.documents.get.callCount).to.equal(1);
                expect(platformMock.documents.get.getCall(0).args).to.deep.equal(
                    [
                        'dpns.domain',
                        {
                            where: [['nameHash', '==', '5620db362ff7ff6021014954f027e989a6ff68d0e7bcf7fed52679847cd9257d45ca']],
                        },
                    ],
                );

                expect(receivedDocument).to.deep.equal(parentDomainDocument);
            });

            it('should return null if domain was not found', async () => {
                platformMock.documents.get.resolves([]);

                const receivedDocument = await resolve.call(platformMock, 'otherName');

                expect(platformMock.documents.get.callCount).to.equal(1);
                expect(platformMock.documents.get.getCall(0).args).to.deep.equal(
                    [
                        'dpns.domain',
                        {
                            where: [['nameHash', '==', '5620f6162fde12d92ef3679f1b61a4d2d78d786d367034b3318a960601af90db95f1']],
                        },
                    ],
                );

                expect(receivedDocument).to.be.null;
            });
        });
    });
});
