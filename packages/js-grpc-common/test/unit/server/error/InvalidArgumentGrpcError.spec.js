const GrpcErrorCodes = require('../../../../lib/server/error/GrpcErrorCodes');
const InvalidArgumentGrpcError = require('../../../../lib/server/error/InvalidArgumentGrpcError');

describe('InvalidArgumentGrpcError', () => {
  let message;
  let metadata;
  let error;

  beforeEach(() => {
    message = 'Message';
    metadata = {};

    error = new InvalidArgumentGrpcError(message, metadata);
  });

  describe('#getMessage', () => {
    it('should return message', () => {
      const result = error.getMessage();

      expect(result).to.equal(message);
    });
  });

  describe('#getCode', () => {
    it('should return INVALID_ARGUMENT error code', () => {
      const result = error.getCode();

      expect(result).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
    });
  });

  describe('#getMetadata', () => {
    it('should return metadata', () => {
      const result = error.getMetadata();

      expect(result).to.equal(metadata);
    });
  });
});
