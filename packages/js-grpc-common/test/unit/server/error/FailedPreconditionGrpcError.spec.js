const GrpcErrorCodes = require('../../../../lib/server/error/GrpcErrorCodes');

const FailedPreconditionGrpcError = require('../../../../lib/server/error/FailedPreconditionGrpcError');

describe('FailedPreconditionGrpcError', () => {
  let message;
  let metadata;
  let error;

  beforeEach(() => {
    message = 'Message';
    metadata = {};

    error = new FailedPreconditionGrpcError(message, metadata);
  });

  describe('#getMessage', () => {
    it('should return message', () => {
      const result = error.getMessage();

      expect(result).to.equal(`Failed precondition: ${message}`);
    });
  });

  describe('#getCode', () => {
    it('should return FAILED_PRECONDITION error code', () => {
      const result = error.getCode();

      expect(result).to.equal(GrpcErrorCodes.FAILED_PRECONDITION);
    });
  });

  describe('#getMetadata', () => {
    it('should return metadata', () => {
      const result = error.getMetadata();

      expect(result).to.equal(metadata);
    });
  });
});
