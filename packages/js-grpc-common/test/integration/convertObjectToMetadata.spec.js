// Import metadata directly to do not import Node.JS server logic in browsers
const { Metadata } = require('@grpc/grpc-js/build/src/metadata');

const convertObjectToMetadata = require('../../lib/convertObjectToMetadata');

describe('convertObjectToMetadata', () => {
  it('should successfully convert an object to Metadata', () => {
    const object = {
      some: 42,
      string: 'someString',
      buffer: Buffer.from('some'),
    };

    const result = convertObjectToMetadata(object);

    expect(result).to.be.an.instanceOf(Metadata);
    // eslint-disable-next-line no-underscore-dangle
    expect(result.internalRepr.get('some')).to.deep.equal(['42']);
    // eslint-disable-next-line no-underscore-dangle
    expect(result.internalRepr.get('string')).to.deep.equal(['"someString"']);
    // eslint-disable-next-line no-underscore-dangle
    expect(result.internalRepr.get('buffer')).to.deep.equal(['{"type":"Buffer","data":[115,111,109,101]}']);
  });
});
