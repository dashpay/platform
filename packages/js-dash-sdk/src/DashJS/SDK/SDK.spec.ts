import { expect } from 'chai';
import {SDK} from "./index";
import 'mocha';
import schema from '../../../tests/fixtures/dp1.schema.json'
const mnemonic = 'agree country attract master mimic ball load beauty join gentle turtle hover';
describe('DashJS - SDK', () => {

  it('should provide expected class', function () {
    expect(SDK.name).to.be.equal('SDK')
    expect(SDK.constructor.name).to.be.equal('Function')
  });
  it('should be instantiable', function () {
    const sdk = new SDK();
    expect(sdk).to.exist;
    expect(sdk.network).to.be.equal('testnet');
    expect(sdk.client.constructor.name).to.be.equal('DAPIClient')
  });
  it('should initiate wallet lib in offline mode without mnemonic', function () {
    const sdk = new SDK();
    expect(sdk.wallet.offlineMode).to.be.equal(true)
  });
  it('should initiate wallet-lib with a mnemonic', function () {
    const sdk = new SDK({mnemonic});
    expect(sdk.wallet.offlineMode).to.be.equal(false)
  });
  it('should initiate platform and only set contract when schema provided', function () {
    const sdkNoSchema= new SDK();
    expect(sdkNoSchema.platform).to.not.have.property('contractId')

    const sdkWithSchema= new SDK({schema});
    expect(sdkWithSchema.platform.contractId).to.equal('4bGwCHfGZYHkAi6ut4Ppm5qSUHSb7zcTFMmLKomrHLcg')
  });
});
