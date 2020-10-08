const getBinaryPropertiesFromSchema = require(
  '../../../lib/dataContract/getBinaryPropertiesFromSchema',
);

describe('getBinaryPropertiesFromSchema', () => {
  let documentSchema;

  beforeEach(() => {
    documentSchema = {
      properties: {
        simple: {
          type: 'string',
        },
        withByteArray: {
          type: 'object',
          byteArray: true,
        },
        nestedObject: {
          type: 'object',
          properties: {
            simple: {
              type: 'string',
            },
            withByteArray: {
              type: 'object',
              byteArray: true,
            },
          },
        },
        arrayOfObject: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              simple: {
                type: 'string',
              },
              withByteArray: {
                type: 'object',
                byteArray: true,
              },
            },
          },
        },
        arrayOfObjects: {
          type: 'array',
          items: [
            {
              type: 'object',
              properties: {
                simple: {
                  type: 'string',
                },
                withByteArray: {
                  type: 'object',
                  byteArray: true,
                },
              },
            },
            {
              type: 'string',
            },
            {
              type: 'array',
              items: [
                {
                  type: 'object',
                  properties: {
                    simple: {
                      type: 'string',
                    },
                    withByteArray: {
                      type: 'object',
                      byteArray: true,
                    },
                  },
                },
              ],
            },
          ],
        },
      },
    };
  });

  it('should return an empty object if not `properties` property found', () => {
    const result = getBinaryPropertiesFromSchema({});

    expect(result).to.deep.equal({});
  });

  it('should return flat object with properties having contentEncoding keyword', () => {
    const result = getBinaryPropertiesFromSchema(documentSchema);

    expect(result).to.deep.equal({
      withByteArray: { type: 'object', byteArray: true },
      'nestedObject.withByteArray': { type: 'object', byteArray: true },
      'arrayOfObject.withByteArray': { type: 'object', byteArray: true },
      'arrayOfObjects[0].withByteArray': { type: 'object', byteArray: true },
      'arrayOfObjects[2][0].withByteArray': { type: 'object', byteArray: true },
    });
  });
});
