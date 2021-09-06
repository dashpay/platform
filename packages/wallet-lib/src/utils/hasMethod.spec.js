const { expect } = require('chai');
const { hasMethod} = require("./index");

describe('Utils - hasMethod', function suite() {
  it('should correctly handle method detection', function () {
      expect(hasMethod({ method1: ()=>null }, 'method1')).to.equal(true);
      expect(hasMethod({ method1: ()=>null }, 'method2')).to.equal(false);
      expect(hasMethod(null, 'method1')).to.equal(false);
  });
});
