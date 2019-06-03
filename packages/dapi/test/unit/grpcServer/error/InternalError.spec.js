const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

const GrpcError = require('../../../../lib/grpcServer/error/GrpcError');
const InternalError = require('../../../../lib/grpcServer/error/InternalError');

use(dirtyChai);

describe('InternalError', () => {
  let error;
  let internalError;

  beforeEach(() => {
    error = new Error();

    internalError = new InternalError(error);
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
