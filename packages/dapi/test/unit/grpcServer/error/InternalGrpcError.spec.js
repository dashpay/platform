const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

const GrpcError = require('../../../../lib/grpcServer/error/GrpcError');
const InternalGrpcError = require('../../../../lib/grpcServer/error/InternalGrpcError');

use(dirtyChai);

describe('InternalGrpcError', () => {
  let error;
  let internalError;

  beforeEach(() => {
    error = new Error();

    internalError = new InternalGrpcError(error);
  });

  describe('#getError', () => {
    it('should return error', () => {
      const result = internalError.getError();

      expect(result).to.equal(error);
    });
  });

  describe('#getCode', () => {
    it('should return INTERNAL error code', () => {
      const result = internalError.getCode();

      expect(result).to.equal(GrpcError.CODES.INTERNAL);
    });
  });
});
