const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCError = require('../../lib/rpcServer/RPCError');
const errorHandlerDecorator = require('../../lib/rpcServer/errorHandlerDecorator');

chai.use(chaiAsPromised);
const { expect } = chai;

describe('lib/rpcServer/errorHandlerDecorator', () => {
  it('should be errorHandlerDecorator function', () => {
    const res = errorHandlerDecorator;
    expect(res).to.be.a('function');
  });
  it('should return function', () => {
    const res = errorHandlerDecorator();
    expect(res).to.be.a('function');
  });
  it('should throw error when call errorHandlerDecorator with non existing command', () => {
    const res = errorHandlerDecorator('fake');
    expect(() => res('my_arg')).to.throw('command is not a function');
  });
  it('Should not modify the error if the error is an instance of an RPCError', () => {
    const throwingFunction = async () => { throw new RPCError(-1, 'Some message'); };
    const decoratedFunction = errorHandlerDecorator(throwingFunction);
    return expect(decoratedFunction()).to.be.rejectedWith(RPCError, 'Some message');
  });
  it('Should throw RPCError with same message in case if it exists ', () => {
    const throwingFunction = async () => { throw new Error('Test message #2'); };
    const decoratedFunction = errorHandlerDecorator(throwingFunction);
    return expect(decoratedFunction()).to.be.rejectedWith(RPCError, 'Test message #2');
  });
});
