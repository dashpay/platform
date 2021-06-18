import { expect } from 'chai';

import resolveByRecord from './resolveByRecord';

describe('Platform', () => {
    describe('Names', () => {
        describe('#resolveByRecord', () => {
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
                    initialize:  this.sinon.stub(),
                };
            });

            it('should resolve domain by it\'s record', async () => {
                platformMock.documents.get.resolves([parentDomainDocument]);

                const receivedDocuments = await resolveByRecord.call(
                    platformMock, 'recordName', 'recordValue',
                );

                expect(platformMock.documents.get.callCount).to.equal(1);
                expect(platformMock.documents.get.getCall(0).args).to.deep.equal([
                    'dpns.domain',
                    {
                        where: [['records.recordName', '==', 'recordValue']],
                    },
                ]);

                expect(receivedDocuments).to.deep.equal([parentDomainDocument]);
            });

            it('should return null if domain was not found', async () => {
                platformMock.documents.get.resolves([]);

                const receivedDocuments = await resolveByRecord.call(
                    platformMock, 'recordName', 'recordValue',
                );

                expect(platformMock.documents.get.callCount).to.equal(1);
                expect(platformMock.documents.get.getCall(0).args).to.deep.equal([
                    'dpns.domain',
                    {
                        where: [['records.recordName', '==', 'recordValue']],
                    },
                ]);

                expect(receivedDocuments).to.be.an('array');
                expect(receivedDocuments.length).to.equal(0);
            });
        });
    });
});
