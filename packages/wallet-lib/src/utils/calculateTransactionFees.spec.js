const { expect } = require('chai');
const { Transaction } = require('@dashevo/dashcore-lib');
const calculateTransactionFees = require('./calculateTransactionFees');

const tx1 = '0300000001b51d5a6f5c7a680bce489e6f5a9b176ac85c49f10db4798867c7d1eb2036fbc3000000006a4730440220283fd42353767188532db4a4f1c3d0a9e96e313196ae1310af6d3006c7aa64ff022027fa50cf065c096f146e00516cb3e28a9bb387a6cf1103aae0592d5c882d25e5012102ba0588ffd3c838b715d7c79bcf1cff2ba69befd5ea52aa3474d66f094536cac0ffffffff0200131a4b000000001976a914838112cc6c85e074aa7f373e942c9f5240c3e13a88ac89959800000000001976a914f728c15b9a5fe4e6d7b6ed74b323e23f5c6e303f88ac00000000';
const tx2 = '0300000001338540c64b794f73913f39f2d42d9139ce7c9d1c0ec5317c62ab4a28d6b0376f000000006b483045022100b996d726d224a762acf8ab3e37c085e796b44960b8e9933571ac57750e8ed05102201c6a36d72f16140d6a152be40add102d95a4ac5b177d300b10c277859690a859012103b5614f077d750a1eaffb23ca188dbcc7e267f4b8ffdedf81cdf970643027191bffffffff02008c8647000000001976a91414b05906daab037707927bc6c83900d5dbf2849688ac09869303000000001976a914791e51fff6554c18216c83d9ca81cf30cc66aff388ac00000000';
describe('Utils - calculateTransactionFees', function suite() {
  it('should ensure a valid transaction', function (){
    const transaction = null
    expect(()=>calculateTransactionFees(transaction)).to.throw('Expected a valid transaction');
  });
  it('should ensure inputs and outputs are provided', function () {
    const transaction = new Transaction(tx2);
    expect(()=>calculateTransactionFees(transaction)).to.throw('Expected transaction input to have the output specified');
  });
  it('should correctly calculate transaction fees', function () {
    const transaction1 = new Transaction(tx1);
    const transaction2 = new Transaction(tx2);
    // We specify the output for the tx2 input.
    transaction2.inputs[0].output = transaction1.outputs[0];
    const fees = calculateTransactionFees(transaction2);
    expect(fees).to.equal(247);
  });

});
