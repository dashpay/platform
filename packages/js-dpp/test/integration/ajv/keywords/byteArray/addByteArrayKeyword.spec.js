const Ajv = require('ajv');

const addByteArrayKeyword = require('../../../../../lib/ajv/keywords/byteArray/addByteArrayKeyword');
const byteArray = require('../../../../../lib/ajv/keywords/byteArray/byteArray');

describe('addByteArrayKeyword', () => {
  let ajv;

  beforeEach(() => {
    ajv = new Ajv();
  });

  it('should add byteArray keyword', () => {
    addByteArrayKeyword(ajv);

    expect(
      ajv.getKeyword('byteArray'),
    ).to.equal(byteArray);
  });

  describe('byteArray', () => {
    beforeEach(() => {
      addByteArrayKeyword(ajv);
    });

    describe('compilation', () => {
      it('should be used with array type', () => {
        const schema = {
          type: 'string',
          byteArray: true,
        };

        ajv.validate(schema, Buffer.alloc(0));

        expect(ajv.errors).to.have.lengthOf(1);
        expect(ajv.errors[0].keyword).to.equal('type');
        expect(ajv.errors[0].params.type).to.equal('string');
      });

      it('should not be used with `items` keyword', () => {
        const schema = {
          type: 'array',
          byteArray: true,
          items: {
            type: 'string',
          },
        };

        try {
          ajv.validate(schema, Buffer.alloc(0));

          expect.fail('should fail with keyword schema error');
        } catch (e) {
          expect(e.message).to.equal('\'byteArray\' should not be used with \'items\'');
        }
      });

      it('should be boolean', () => {
        const schema = {
          type: 'array',
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
          type: 'array',
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
      it('should accept array of integers', () => {
        const schema = {
          type: 'array',
          byteArray: true,
        };

        ajv.validate(schema, ['string']);

        expect(ajv.errors).to.have.lengthOf(2);

        const [error, byteArrayError] = ajv.errors;

        expect(error.keyword).to.equal('type');
        expect(error.schemaPath).to.equal('#/items/type');
        expect(error.message).to.equal('should be integer');

        expect(byteArrayError.keyword).to.equal('byteArray');
      });

      it('should accept array of integers not less than 0', () => {
        const schema = {
          type: 'array',
          byteArray: true,
        };

        ajv.validate(schema, [-1]);

        expect(ajv.errors).to.have.lengthOf(2);

        const [error, byteArrayError] = ajv.errors;

        expect(error.keyword).to.equal('minimum');
        expect(error.schemaPath).to.equal('#/items/minimum');
        expect(error.message).to.equal('should be >= 0');

        expect(byteArrayError.keyword).to.equal('byteArray');
      });

      it('should accept array of integers not greater than 255', () => {
        const schema = {
          type: 'array',
          byteArray: true,
        };

        ajv.validate(schema, [0, 256]);

        expect(ajv.errors).to.have.lengthOf(2);

        const [error, byteArrayError] = ajv.errors;

        expect(error.keyword).to.equal('maximum');
        expect(error.schemaPath).to.equal('#/items/maximum');
        expect(error.message).to.equal('should be <= 255');

        expect(byteArrayError.keyword).to.equal('byteArray');
      });
    });
  });
});
