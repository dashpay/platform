const { Metadata } = require('grpc');

const convertObjectToMetadata = require('../../lib/convertObjectToMetadata');

describe('convertObjectToMerata', () => {
  it('should successfully convert an object to Metadata', () => {
    const object = {
      some: 42,
      string: 'someString',
      buffer: Buffer.from('some'),
    };

    const result = convertObjectToMetadata(object);

    expect(result).to.be.an.instanceOf(Metadata);
    // eslint-disable-next-line no-underscore-dangle
    expect(result._internal_repr.some).to.deep.equal(['42']);
    // eslint-disable-next-line no-underscore-dangle
    expect(result._internal_repr.string).to.deep.equal(['"someString"']);
    // eslint-disable-next-line no-underscore-dangle
    expect(result._internal_repr.buffer).to.deep.equal(['{"type":"Buffer","data":[115,111,109,101]}']);
  });
});
