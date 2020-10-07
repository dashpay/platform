const Document = require('../../../lib/document/Document');
const EncodedBuffer = require('../../../lib/util/encoding/EncodedBuffer');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

describe('Document', () => {
  let document;
  let dataContract;

  beforeEach(() => {
    dataContract = getDataContractFixture();
    [document] = getDocumentsFixture(dataContract).slice(8);
  });

  describe('#toJSON', () => {
    it('should return raw document with encoded binary fields', () => {
      const result = document.toJSON();

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId().toString(),
        $id: document.getId().toString(),
        $ownerId: EncodedBuffer.from(
          getDocumentsFixture.ownerId, EncodedBuffer.ENCODING.BASE58,
        ).toString(),
        $revision: 1,
        $type: 'withContentEncoding',
        base64Field: document.getData().base64Field.toString(),
        base58Field: document.getData().base58Field.toString(),
      });
    });
  });

  describe('#toObject', () => {
    it('should return raw document', () => {
      const result = document.toObject();

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId(),
        $id: document.getId(),
        $ownerId: getDocumentsFixture.ownerId,
        $revision: 1,
        $type: 'withContentEncoding',
        base64Field: document.get('base64Field'),
        base58Field: document.get('base58Field'),
      });
    });

    it('should return raw document with binary data as instance of EncodedBuffer', () => {
      const result = document.toObject({ encodedBuffer: true });

      expect(result).to.deep.equal({
        $protocolVersion: document.getProtocolVersion(),
        $dataContractId: dataContract.getId(),
        $id: document.getId(),
        $ownerId: getDocumentsFixture.ownerId,
        $revision: 1,
        $type: 'withContentEncoding',
        base64Field: document.getData().base64Field,
        base58Field: document.getData().base58Field,
      });

      expect(result.base64Field).to.be.an.instanceOf(EncodedBuffer);
      expect(result.base58Field).to.be.an.instanceOf(EncodedBuffer);
    });
  });

  describe('#fromJSON', () => {
    it('should return new document from raw document in JSON format', () => {
      const result = Document.fromJSON(document.toJSON(), dataContract);

      expect(result.toObject()).to.deep.equal(document.toObject());
    });
  });
});
