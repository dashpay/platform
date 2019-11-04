const getPropertyDefinitionByPath = require(
  '../../../lib/dataContract/getPropertyDefinitionByPath',
);

describe('getPropertyDefinitionByPath', () => {
  let schema;

  beforeEach(() => {
    schema = {
      properties: {
        a: {
          type: 'string',
        },
        b: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              inner: {
                type: 'object',
                patternProperties: {
                  '[a-z]': {
                    type: 'string',
                  },
                },
              },
            },
          },
        },
        c: {
          type: 'object',
          properties: {
            inner: {
              type: 'object',
              patternProperties: {
                '[a-z]': {
                  type: 'string',
                },
              },
            },
          },
        },
      },
    };
  });

  it('should return `undefined` if property not found', () => {
    const definition = getPropertyDefinitionByPath(schema, 'nope');

    expect(definition).to.be.undefined();
  });

  it('should return definition immediately if path is one-level', () => {
    const definition = getPropertyDefinitionByPath(schema, 'a');

    expect(definition).to.deep.equal({
      type: 'string',
    });
  });

  it('should return nested definition from an array', () => {
    const definition = getPropertyDefinitionByPath(schema, 'b.inner');

    expect(definition).to.deep.equal(schema.properties.b.items.properties.inner);
  });

  it('should return nested definition from object', () => {
    const definition = getPropertyDefinitionByPath(schema, 'c.inner');

    expect(definition).to.deep.equal(schema.properties.c.properties.inner);
  });

  it('should return definition that match a pattern', () => {
    const definition = getPropertyDefinitionByPath(schema, 'c.inner.some');

    expect(definition).to.deep.equal({
      type: 'string',
    });
  });

  it('should return `undefined` if path does not match a pattern', () => {
    const definition = getPropertyDefinitionByPath(schema, 'c.inner.NOPE');

    expect(definition).to.be.undefined();
  });

  it('should return `undefined` if first item in a path is not an object or array', () => {
    const definition = getPropertyDefinitionByPath(schema, 'a.someOther');

    expect(definition).to.be.undefined();
  });
});
