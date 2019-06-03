const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

const GrpcError = require('../../../../lib/grpcServer/error/GrpcError');
const InvalidArgumentError = require('../../../../lib/grpcServer/error/InvalidArgumentError');

use(dirtyChai);

describe('InvalidArgumentError', () => {
  let message;
  let metadata;
  let error;

  beforeEach(() => {
    message = 'Message';
    metadata = {};

    error = new InvalidArgumentError(message, metadata);
  });

  describe('#getMessage', () => {
    it('should return message', () => {
      const result = error.getMessage();

      expect(result).to.equal(`Invalid argument: ${message}`);
    });
  });

  describe('#getCode', () => {
    it('should return INVALID_ARGUMENT error code', () => {
      const result = error.getCode();

      expect(result).to.equal(GrpcError.CODES.INVALID_ARGUMENT);
    });
  });

  describe('#getMetadata', () => {
    it('should return metadata', () => {
      const result = error.getMetadata();

      expect(result).to.equal(metadata);
    });
  });
});
