const chai = require('chai');
const RPCError = require('../../../lib/rpcServer/RPCError');

const { expect } = chai;

describe('lib/rpcServer/RPCError', () => {
  describe('#factory', () => {
    it('should create RPCError instance without params', () => {
      const res = new RPCError();
      expect(res).to.be.instanceof(RPCError);
    });
  });
  describe('#factory', () => {
    it('should create RPCError instance with code', () => {
      const res = new RPCError(200);
      expect(res).to.be.instanceof(RPCError);
    });
  });
  describe('#factory', () => {
    it('should create RPCError instance with code & message', () => {
      const res = new RPCError(200, 'my_message');
      expect(res).to.be.instanceof(RPCError);
    });
  });
  describe('#factory', () => {
    it('should create RPCError instance with code, message and data', () => {
      const data = {};
      const res = new RPCError(200, 'my_message', data);
      expect(res).to.be.instanceof(RPCError);
    });
  });
  describe('#factory', () => {
    it('should create RPCError instance with code, message, data and originalStack', () => {
      const data = {};
      const res = new RPCError(200, 'my_message', data, 'my_stack');
      expect(res).to.be.instanceof(RPCError);
    });
  });
});
