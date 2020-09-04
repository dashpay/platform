const getEncodedPropertiesFromSchema = require(
  '../../../lib/dataContract/getEncodedPropertiesFromSchema',
);

describe('getEncodedPropertiesFromSchema', () => {
  let documentSchema;

  beforeEach(() => {
    documentSchema = {
      properties: {
        simple: {
          type: 'string',
        },
        withEncoding: {
          type: 'string',
          contentEncoding: 'base64',
        },
        nestedObject: {
          type: 'object',
          properties: {
            simple: {
              type: 'string',
            },
            withEncoding: {
              type: 'string',
              contentEncoding: 'base64',
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
              withEncoding: {
                type: 'string',
                contentEncoding: 'base64',
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
                withEncoding: {
                  type: 'string',
                  contentEncoding: 'base64',
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
                    withEncoding: {
                      type: 'string',
                      contentEncoding: 'base64',
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
    const result = getEncodedPropertiesFromSchema({});

    expect(result).to.deep.equal({});
  });

  it('should return flat object with properties having contentEncoding keyword', () => {
    const result = getEncodedPropertiesFromSchema(documentSchema);

    expect(result).to.deep.equal({
      withEncoding: { type: 'string', contentEncoding: 'base64' },
      'nestedObject.withEncoding': { type: 'string', contentEncoding: 'base64' },
      'arrayOfObject.withEncoding': { type: 'string', contentEncoding: 'base64' },
      'arrayOfObjects[0].withEncoding': { type: 'string', contentEncoding: 'base64' },
      'arrayOfObjects[2][0].withEncoding': { type: 'string', contentEncoding: 'base64' },
    });
  });
});
