import { expect } from 'chai';

import resolve from './resolve';

describe('Platform', () => {
  describe('Names', () => {
    describe('#resolve', () => {
      let platformMock;
      let parentDomainDocument;

      beforeEach(async function beforeEach() {
        parentDomainDocument = { label: 'parent' };

        platformMock = {
          documents: {
            get: this.sinon.stub(),
          },
          initialize: this.sinon.stub(),
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
              where: [
                ['normalizedParentDomainName', '==', ''],
                ['normalizedLabel', '==', 'parent'],
              ],
            },
          ],
        );

        expect(receivedDocument).to.deep.equal(parentDomainDocument);
      });

      it('should return null if domain was not found', async () => {
        platformMock.documents.get.resolves([]);

        const receivedDocument = await resolve.call(platformMock, 'otherName.parent');

        expect(platformMock.documents.get.callCount).to.equal(1);
        expect(platformMock.documents.get.getCall(0).args).to.deep.equal(
          [
            'dpns.domain',
            {
              where: [
                ['normalizedParentDomainName', '==', 'parent'],
                ['normalizedLabel', '==', '0thername'],
              ],
            },
          ],
        );

        expect(receivedDocument).to.be.null;
      });
    });
  });
});
