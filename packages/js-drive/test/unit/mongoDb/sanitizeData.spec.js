const { sanitize, unsanitize, INTERNAL_PREFIX } = require('../../../lib/mongoDb/sanitizer');

describe('sanitizeData', () => {
  let unsanitizedData;
  let sanitizedData;

  beforeEach(() => {
    sanitizedData = {
      [`${INTERNAL_PREFIX}$a`]: {
        a: 1,
        b: 2,
        [`${INTERNAL_PREFIX}$c`]: {
          a: 1,
          b: 2,
        },
      },
      b: [
        { [`${INTERNAL_PREFIX}$a`]: 1 },
        { b: 2 },
        { c: 3 },
      ],
      c: 3,
    };

    unsanitizedData = {
      $a: {
        a: 1,
        b: 2,
        $c: {
          a: 1,
          b: 2,
        },
      },
      b: [
        { $a: 1 },
        { b: 2 },
        { c: 3 },
      ],
      c: 3,
    };
  });

  describe('sanitize', () => {
    it('should add an internal prefix to dollar-prefixed fields', () => {
      const result = sanitize(unsanitizedData);

      expect(result).to.deep.equal(sanitizedData);
    });
  });

  describe('unsanitize', () => {
    it('should remove an internal prefix from any dollar-prefixed fields', () => {
      const result = unsanitize(sanitizedData);

      expect(result).to.deep.equal(unsanitizedData);
    });
  });
});
