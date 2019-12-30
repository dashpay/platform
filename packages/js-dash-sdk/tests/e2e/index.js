const {expect} = require('chai');
const DashJS = require('../../dist/dash.min');

describe('DashJS', () => {

  it('should provide expected class', function () {
    expect(DashJS).to.have.property('SDK');
    expect(DashJS.SDK.constructor.name).to.be.equal('Function')
  });
});
