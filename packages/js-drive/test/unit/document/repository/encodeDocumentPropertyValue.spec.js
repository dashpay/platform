const encodeDocumentPropertyValue = require('../../../../lib/document/repository/encodeDocumentPropertyValue');

describe('encodeDocumentPropertyValue', () => {
  describe('integer', () => {
    let propertyDefinition;

    beforeEach(() => {
      propertyDefinition = {
        type: 'integer',
      };
    });

    it('should encode and compare positive integers', () => {
      const integer1 = 1;
      const integer2 = 600;
      const integer3 = Number.MAX_SAFE_INTEGER;

      const encodedInteger1 = encodeDocumentPropertyValue(integer1, propertyDefinition);
      const encodedInteger2 = encodeDocumentPropertyValue(integer2, propertyDefinition);
      const encodedInteger3 = encodeDocumentPropertyValue(integer3, propertyDefinition);

      expect(encodedInteger1).to.deep.equal(Buffer.from('8000000000000001', 'hex'));
      expect(encodedInteger2).to.deep.equal(Buffer.from('8000000000000258', 'hex'));
      expect(encodedInteger3).to.deep.equal(Buffer.from('801fffffffffffff', 'hex'));

      expect(encodedInteger1.compare(encodedInteger2)).to.equal(-1);
      expect(encodedInteger2.compare(encodedInteger3)).to.equal(-1);
    });

    it('should encode and compare negative integers', () => {
      const integer1 = -1;
      const integer2 = -600;
      const integer3 = Number.MIN_SAFE_INTEGER;

      const encodedInteger1 = encodeDocumentPropertyValue(integer1, propertyDefinition);
      const encodedInteger2 = encodeDocumentPropertyValue(integer2, propertyDefinition);
      const encodedInteger3 = encodeDocumentPropertyValue(integer3, propertyDefinition);

      expect(encodedInteger1).to.deep.equal(Buffer.from('7fffffffffffffff', 'hex'));
      expect(encodedInteger2).to.deep.equal(Buffer.from('7ffffffffffffda8', 'hex'));
      expect(encodedInteger3).to.deep.equal(Buffer.from('7fe0000000000001', 'hex'));

      expect(encodedInteger1.compare(encodedInteger2)).to.equal(1);
      expect(encodedInteger2.compare(encodedInteger3)).to.equal(1);
    });

    it('should check that zero is in the middle between positive and negative', () => {
      const integer1 = -1;
      const integer2 = 0;
      const integer3 = 1;

      const encodedInteger1 = encodeDocumentPropertyValue(integer1, propertyDefinition);
      const encodedInteger2 = encodeDocumentPropertyValue(integer2, propertyDefinition);
      const encodedInteger3 = encodeDocumentPropertyValue(integer3, propertyDefinition);

      expect(encodedInteger1).to.deep.equal(Buffer.from('7fffffffffffffff', 'hex'));
      expect(encodedInteger2).to.deep.equal(Buffer.from('8000000000000000', 'hex'));
      expect(encodedInteger3).to.deep.equal(Buffer.from('8000000000000001', 'hex'));

      expect(encodedInteger2.compare(encodedInteger1)).to.equal(1);
      expect(encodedInteger2.compare(encodedInteger3)).to.equal(-1);
      expect(encodedInteger3.compare(encodedInteger1)).to.equal(1);
    });
  });

  describe('number', () => {
    let propertyDefinition;

    beforeEach(() => {
      propertyDefinition = {
        type: 'number',
      };
    });

    it('should encode and compare numbers', () => {
      const number1 = 1.0;
      const number2 = 23.65;
      const number3 = 1394.584;
      const number4 = Number.MAX_VALUE;
      const number5 = Number.POSITIVE_INFINITY;

      const encodedNumber1 = encodeDocumentPropertyValue(number1, propertyDefinition);
      const encodedNumber2 = encodeDocumentPropertyValue(number2, propertyDefinition);
      const encodedNumber3 = encodeDocumentPropertyValue(number3, propertyDefinition);
      const encodedNumber4 = encodeDocumentPropertyValue(number4, propertyDefinition);
      const encodedNumber5 = encodeDocumentPropertyValue(number5, propertyDefinition);

      expect(encodedNumber1).to.deep.equal(Buffer.from('bff0000000000000', 'hex'));
      expect(encodedNumber2).to.deep.equal(Buffer.from('c037a66666666666', 'hex'));
      expect(encodedNumber3).to.deep.equal(Buffer.from('c095ca5604189375', 'hex'));
      expect(encodedNumber4).to.deep.equal(Buffer.from('ffefffffffffffff', 'hex'));
      expect(encodedNumber5).to.deep.equal(Buffer.from('fff0000000000000', 'hex'));

      expect(encodedNumber1.compare(encodedNumber2)).to.equal(-1);
      expect(encodedNumber2.compare(encodedNumber3)).to.equal(-1);
      expect(encodedNumber3.compare(encodedNumber4)).to.equal(-1);
      expect(encodedNumber4.compare(encodedNumber5)).to.equal(-1);
    });

    it('should check that zero is in the middle between positive and negative', () => {
      const number1 = 0.0 - Number.EPSILON;
      const number2 = 0.0;
      const number3 = 0.0 + Number.EPSILON;

      const encodedNumber1 = encodeDocumentPropertyValue(number1, propertyDefinition);
      const encodedNumber2 = encodeDocumentPropertyValue(number2, propertyDefinition);
      const encodedNumber3 = encodeDocumentPropertyValue(number3, propertyDefinition);

      expect(encodedNumber1).to.deep.equal(Buffer.from('434fffffffffffff', 'hex'));
      expect(encodedNumber2).to.deep.equal(Buffer.from('8000000000000000', 'hex'));
      expect(encodedNumber3).to.deep.equal(Buffer.from('bcb0000000000000', 'hex'));

      expect(encodedNumber2.compare(encodedNumber1)).to.equal(1);
      expect(encodedNumber2.compare(encodedNumber3)).to.equal(-1);
      expect(encodedNumber3.compare(encodedNumber1)).to.equal(1);
    });
  });

  describe('boolean', () => {
    let propertyDefinition;

    beforeEach(() => {
      propertyDefinition = {
        type: 'boolean',
      };
    });

    it('should encode boolean values', () => {
      const boolean1 = false;
      const boolean2 = true;

      const encodedBoolean1 = encodeDocumentPropertyValue(boolean1, propertyDefinition);
      const encodedBoolean2 = encodeDocumentPropertyValue(boolean2, propertyDefinition);

      expect(encodedBoolean1).to.deep.equal(Buffer.from('00', 'hex'));
      expect(encodedBoolean2).to.deep.equal(Buffer.from('01', 'hex'));
    });
  });

  describe('array', () => {
    let propertyDefinition;

    beforeEach(() => {
      propertyDefinition = {
        type: 'array',
      };
    });

    it('should encode byte array', () => {
      propertyDefinition.byteArray = true;

      const byteArray = Uint8Array.from([1, 2, 3]);

      const encodedByteArray = encodeDocumentPropertyValue(byteArray, propertyDefinition);

      expect(encodedByteArray).to.be.an.instanceOf(Buffer);
      expect(encodedByteArray).to.deep.equal(Buffer.from([1, 2, 3]));
    });
  });
});
