const { encode } = require('../../../lib/util/serializer');

const DataSerializationError = require('../../../lib/util/errors/MaxEncodedBytesReachedError');

describe('serializer', function main() {
  this.timeout(10000);

  describe('#encode', () => {
    it('should throw an error if payload is larger that 16 Kb', () => {
      const payload = {};
      for (let i = 0; i < 10000; i++) {
        payload[i] = i;
      }

      try {
        encode(payload);
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(DataSerializationError);
        expect(e.getPayload()).to.deep.equal(payload);
      }
    });
  });
});
