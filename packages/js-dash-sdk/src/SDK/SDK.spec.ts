import { expect } from 'chai';
import SDK from "./index";
import 'mocha';

describe('Dash', () => {

  it('should provide expected class', function () {
    expect(SDK).to.have.property('Client');
    expect(SDK.Client.name).to.be.equal('Client')
    expect(SDK.Client.constructor.name).to.be.equal('Function')
  });
});
