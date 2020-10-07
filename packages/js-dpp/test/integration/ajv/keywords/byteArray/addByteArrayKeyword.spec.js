const Ajv = require('ajv');

const addByteArrayKeyword = require('../../../../../lib/ajv/keywords/byteArray/addByteArrayKeyword');
const byteArrayKeyword = require('../../../../../lib/ajv/keywords/byteArray/byteArray');
const minBytesLength = require('../../../../../lib/ajv/keywords/byteArray/minBytesLength');
const maxBytesLength = require('../../../../../lib/ajv/keywords/byteArray/maxBytesLength');

describe('addByteArrayKeyword', () => {
  let ajv;

  beforeEach(() => {
    ajv = new Ajv();
  });

  it('should add byteArray, minBytesLength and maxBytesLength keywords', () => {
    addByteArrayKeyword(ajv);

    expect(
      ajv.getKeyword('byteArray'),
    ).to.equal(byteArrayKeyword);

    expect(
      ajv.getKeyword('minBytesLength'),
    ).to.equal(minBytesLength);

    expect(
      ajv.getKeyword('maxBytesLength'),
    ).to.equal(maxBytesLength);
  });

  describe('byteArray', () => {
    beforeEach(() => {
      addByteArrayKeyword(ajv);
    });

    describe('compilation', () => {
      it('should be used with object type', () => {
        const schema = {
          type: 'string',
          byteArray: true,
        };

        ajv.validate(schema, Buffer.alloc(0));

        expect(ajv.errors).to.have.lengthOf(1);
        expect(ajv.errors[0].keyword).to.equal('type');
        expect(ajv.errors[0].params.type).to.equal('string');
      });

      it('should be boolean', () => {
        const schema = {
          type: 'object',
          byteArray: 'something',
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be boolean');
        }
      });

      it('should have value of true', () => {
        const schema = {
          type: 'object',
          byteArray: false,
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be equal to constant');
        }
      });
    });

    describe('validation', () => {
      it('should invalidate everything but Buffer', () => {
        const schema = {
          type: 'object',
          byteArray: true,
        };

        ajv.validate(schema, { });

        expect(ajv.errors).to.have.lengthOf(1);

        const [error] = ajv.errors;

        expect(error.keyword).to.equal('byteArray');
        expect(error.message).to.equal('should be a byte array');
      });

      it('should accept Buffer', () => {
        const schema = {
          type: 'object',
          byteArray: true,
        };

        const result = ajv.validate(schema, Buffer.alloc(0));

        expect(result).to.be.true();
      });
    });
  });

  describe('minBytesLength', () => {
    beforeEach(() => {
      addByteArrayKeyword(ajv);
    });

    describe('compilation', () => {
      it('should be used with object type', () => {
        const schema = {
          byteArray: true,
          type: 'string',
          minBytesLength: 1,
        };

        ajv.validate(schema, Buffer.alloc(0));

        expect(ajv.errors).to.have.lengthOf(1);

        const [error] = ajv.errors;

        expect(error.keyword).to.equal('type');
        expect(error.params.type).to.equal('string');
      });

      it('should be used together with byteArray', () => {
        const schema = {
          type: 'object',
          minBytesLength: 1,
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('parent schema must have all required keywords: byteArray');
        }
      });

      it('should be an integer', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          minBytesLength: 'something',
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be integer');
        }
      });

      it('should not be less than 0', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          minBytesLength: -1,
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be >= 0');
        }
      });
    });

    describe('validation', () => {
      it('should invalidate a byte array shorter than specified', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          minBytesLength: 2,
        };

        ajv.validate(schema, Buffer.alloc(1));

        expect(ajv.errors).to.have.lengthOf(1);

        const [error] = ajv.errors;

        expect(error.keyword).to.equal('minBytesLength');
        expect(error.message).to.equal('should NOT be shorter than 2 bytes');
        expect(error.params.limit).to.equal(2);
      });

      it('should accept byte array longer than specified', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          minBytesLength: 2,
        };

        const result = ajv.validate(schema, Buffer.alloc(2));

        expect(result).to.be.true();
      });
    });
  });

  describe('maxBytesLength', () => {
    beforeEach(() => {
      addByteArrayKeyword(ajv);
    });

    describe('compilation', () => {
      it('should be used with object type', () => {
        const schema = {
          byteArray: true,
          type: 'string',
          maxBytesLength: 1,
        };

        ajv.validate(schema, Buffer.alloc(0));

        expect(ajv.errors).to.have.lengthOf(1);

        const [error] = ajv.errors;

        expect(error.keyword).to.equal('type');
        expect(error.params.type).to.equal('string');
      });

      it('should be used together with byteArray', () => {
        const schema = {
          type: 'object',
          maxBytesLength: 1,
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('parent schema must have all required keywords: byteArray');
        }
      });

      it('should be an integer', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          maxBytesLength: 'something',
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be integer');
        }
      });

      it('should not be less than 0', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          maxBytesLength: -1,
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('keyword schema is invalid: data should be >= 0');
        }
      });
    });

    describe('validation', () => {
      it('should invalidate a byte array shorter than specified', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          maxBytesLength: 2,
        };

        ajv.validate(schema, Buffer.alloc(3));

        expect(ajv.errors).to.have.lengthOf(1);

        const [error] = ajv.errors;

        expect(error.keyword).to.equal('maxBytesLength');
        expect(error.message).to.equal('should NOT be longer than 2 bytes');
        expect(error.params.limit).to.equal(2);
      });

      it('should accept byte array shorter than specified', () => {
        const schema = {
          type: 'object',
          byteArray: true,
          maxBytesLength: 2,
        };

        const result = ajv.validate(schema, Buffer.alloc(1));

        expect(result).to.be.true();
      });
    });
  });
});
