require('../../_before.js');
const should = require('should');

describe('Insight-API - getBlockHeaders', function() {
    it('should return the next 25 blocks from height', function() {
        //Given the fact that the expectHeaders array will have confirmations outdated (they equal the time when we import it)
        //We will first modify the value so we can determine the actual one
        return SDK.Explorer.API.getLastBlock()
            .then(getLastBlock => {
                let expectedFirstConfNb = (getLastBlock.height - validBlockHeight) + 1;
                expectHeaders.filter(function(_el, i) {
                    _el.confirmations = expectedFirstConfNb - i;
                })

                return SDK.Explorer.API.getBlockHeaders(validBlockHeight);

            })
            .then(blockHeaders => {
                blockHeaders.should.deepEqual(expectHeaders);
            })
            .catch(err => console.log(err))
    });

    it('should return the next 25 blocks from hash', function() {
        return SDK.Explorer.API.getBlockHeaders(validBlockHash)
            .then(blockHeaders => {
                blockHeaders.should.deepEqual(expectHeaders);
            })

    });
});

let validBlockHash = "00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd";
let validBlockHeight = 195460;
let expectHeaders = [{
    hash: '00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd',
    version: 536870912,
    confirmations: 31,
    height: 195460,
    chainWork: '0000000000000000000000000000000000000000000000000000567c1242e904',
    prevHash: '00000000f668c3b80b1e1d667ea401ce6536cbb89e77326709906db0b7309e1c',
    nextHash: '000000006147f78eea9469aca025a456f32adfb14ce8e721814c2f20b8b0b0e8',
    merkleRoot: 'e270a45c438fb8befead6b3c9e88201b0aa96e5f43920fb78fdb8dbb3e433e04',
    time: 1493937194,
    medianTime: 1493936616,
    nonce: 1704653590,
    bits: '1d01000d',
    difficulty: 0.9997864193198981
},
{
    hash: '000000006147f78eea9469aca025a456f32adfb14ce8e721814c2f20b8b0b0e8',
    version: 536870912,
    confirmations: 30,
    height: 195461,
    chainWork: '0000000000000000000000000000000000000000000000000000567d1166a728',
    prevHash: '00000000e77f43412f0a536c1a02cc7ca4c84ce812122a0a4efe6bef386ee8dd',
    nextHash: '000000008586d7ca859d598130ff25dde8a02d333fd068ba1905d49fc605fe22',
    merkleRoot: 'e29567cfa436ab2dc4c9a504e5b8607b56708a1e8abc11ec8a9b72e36ba6f54a',
    time: 1493937518,
    medianTime: 1493936643,
    nonce: 1569314049,
    bits: '1d0100dd',
    difficulty: 0.9966239335736119
},
{
    hash: '000000008586d7ca859d598130ff25dde8a02d333fd068ba1905d49fc605fe22',
    version: 536870912,
    confirmations: 29,
    height: 195462,
    chainWork: '0000000000000000000000000000000000000000000000000000567dff0d54aa',
    prevHash: '000000006147f78eea9469aca025a456f32adfb14ce8e721814c2f20b8b0b0e8',
    nextHash: '000000008a7abe5cdc04593e469461fe36adaf01193b75b35c8fffabf9eb170f',
    merkleRoot: '419f79ad16ffa32158c8eeec0107623e9da6fcb4b88bd25efe4f7a9c9a0fd485',
    time: 1493937799,
    medianTime: 1493936804,
    nonce: 1045558025,
    bits: '1d0113c4',
    difficulty: 0.9283103858575557
},
{
    hash: '000000008a7abe5cdc04593e469461fe36adaf01193b75b35c8fffabf9eb170f',
    version: 536870912,
    confirmations: 28,
    height: 195463,
    chainWork: '0000000000000000000000000000000000000000000000000000567ee11fa095',
    prevHash: '000000008586d7ca859d598130ff25dde8a02d333fd068ba1905d49fc605fe22',
    nextHash: '000000011d72350ac49db2b2ef92413dfd0a1997de04964eb71f2d01ffdca240',
    merkleRoot: '9f7a5d1ce902ba7b7ae6ebef83d8bcaeae3f1719f45fd55607e7c624ad3536ab',
    time: 1493938151,
    medianTime: 1493936867,
    nonce: 4167768599,
    bits: '1d0121e4',
    difficulty: 0.8830782083760039
},
{
    hash: '000000011d72350ac49db2b2ef92413dfd0a1997de04964eb71f2d01ffdca240',
    version: 536870912,
    confirmations: 27,
    height: 195464,
    chainWork: '0000000000000000000000000000000000000000000000000000567fb38b2cbc',
    prevHash: '000000008a7abe5cdc04593e469461fe36adaf01193b75b35c8fffabf9eb170f',
    nextHash: '00000000e6c2d95b4234adb7adeb69cd284957cf213f9ea54195acc2dbcd52eb',
    merkleRoot: '6cb159a3a03757e22b6e05a5f8b5d63e612494f92cf50d132815131afc701be7',
    time: 1493938585,
    medianTime: 1493937042,
    nonce: 3361344002,
    bits: '1d013774',
    difficulty: 0.8219410023578989
},
{
    hash: '00000000e6c2d95b4234adb7adeb69cd284957cf213f9ea54195acc2dbcd52eb',
    version: 536870912,
    confirmations: 26,
    height: 195465,
    chainWork: '000000000000000000000000000000000000000000000000000056807414ff64',
    prevHash: '000000011d72350ac49db2b2ef92413dfd0a1997de04964eb71f2d01ffdca240',
    nextHash: '000000003bfad6857e207f5db609f820c1c9e40f735aa746cd1afcb43a258459',
    merkleRoot: 'ba5849aea72ecff4be6c4f63b51c21b5e30cb9825cb364e49f1074907e259063',
    time: 1493938590,
    medianTime: 1493937194,
    nonce: 1844746519,
    bits: '1d015461',
    difficulty: 0.7520915340211392
},
{
    hash: '000000003bfad6857e207f5db609f820c1c9e40f735aa746cd1afcb43a258459',
    version: 536870912,
    confirmations: 25,
    height: 195466,
    chainWork: '0000000000000000000000000000000000000000000000000000568136c47b10',
    prevHash: '00000000e6c2d95b4234adb7adeb69cd284957cf213f9ea54195acc2dbcd52eb',
    nextHash: '000000005ff3e597067b88ace718dfdcfa8933f536f7cb3609f0b02ca6c6b2ab',
    merkleRoot: 'af0e1573db5614e632f5a93edd795713a02e5de22892628026a7a9dab7a24221',
    time: 1493938638,
    medianTime: 1493937518,
    nonce: 3457444352,
    bits: '1d0150a0',
    difficulty: 0.7604785555142963
},
{
    hash: '000000005ff3e597067b88ace718dfdcfa8933f536f7cb3609f0b02ca6c6b2ab',
    version: 536870912,
    confirmations: 24,
    height: 195467,
    chainWork: '00000000000000000000000000000000000000000000000000005681fedc5828',
    prevHash: '000000003bfad6857e207f5db609f820c1c9e40f735aa746cd1afcb43a258459',
    nextHash: '00000000e1cef17e3d1df9dbceeb8217528f278c5ddfba4b058fa11a1d108042',
    merkleRoot: '6652e2e59f0a27fbc5ebdebcb11931f069ea5a71d09d32e71af01767aee3d682',
    time: 1493938915,
    medianTime: 1493937799,
    nonce: 1440913178,
    bits: '1d014787',
    difficulty: 0.7816022040144549
},
{
    hash: '00000000e1cef17e3d1df9dbceeb8217528f278c5ddfba4b058fa11a1d108042',
    version: 536870912,
    confirmations: 23,
    height: 195468,
    chainWork: '00000000000000000000000000000000000000000000000000005682c16238ab',
    prevHash: '000000005ff3e597067b88ace718dfdcfa8933f536f7cb3609f0b02ca6c6b2ab',
    nextHash: '00000000a64638be92b247084837f500c45c015e468afb6998e4e558d36fef09',
    merkleRoot: '37e46cca1ff94957366b0499f207d7831e1fb7d7a389b22bd78a8d67e85876bc',
    time: 1493938970,
    medianTime: 1493938151,
    nonce: 3968249623,
    bits: '1d0150e8',
    difficulty: 0.759843706520731
},
{
    hash: '00000000a64638be92b247084837f500c45c015e468afb6998e4e558d36fef09',
    version: 536870912,
    confirmations: 22,
    height: 195469,
    chainWork: '00000000000000000000000000000000000000000000000000005683827b00f4',
    prevHash: '00000000e1cef17e3d1df9dbceeb8217528f278c5ddfba4b058fa11a1d108042',
    nextHash: '000000001fec9fdaf6bc78fd8f8778f824b6786754bed259ab93ab57eb7680ef',
    merkleRoot: '457e429e33c1f95009a5325b12d81182935f92b1fcc77a2e5719f466cc1961be',
    time: 1493939019,
    medianTime: 1493938585,
    nonce: 1009552641,
    bits: '1d015365',
    difficulty: 0.7542728894515739
},
{
    hash: '000000001fec9fdaf6bc78fd8f8778f824b6786754bed259ab93ab57eb7680ef',
    version: 536870912,
    confirmations: 21,
    height: 195470,
    chainWork: '0000000000000000000000000000000000000000000000000000568448d2240c',
    prevHash: '00000000a64638be92b247084837f500c45c015e468afb6998e4e558d36fef09',
    nextHash: '00000000ca292082e7175f3d6f6862207611539ffd5017611c57373803d16a7d',
    merkleRoot: '225197a0942d3b1c2b9fe9ffc534c21b2553d129fceec9b79d34151064ca86cc',
    time: 1493939078,
    medianTime: 1493938590,
    nonce: 2211601925,
    bits: '1d014a6c',
    difficulty: 0.7747552844375089
},
{
    hash: '00000000ca292082e7175f3d6f6862207611539ffd5017611c57373803d16a7d',
    version: 536870912,
    confirmations: 20,
    height: 195471,
    chainWork: '000000000000000000000000000000000000000000000000000056851c790155',
    prevHash: '000000001fec9fdaf6bc78fd8f8778f824b6786754bed259ab93ab57eb7680ef',
    nextHash: '000000009ffc1f3135d785e7fb749df33ee630091519da3adfbe89d65adeae09',
    merkleRoot: '22cb1a50ec0b11c6a0d23376d1a176e72ed5bedeac7b21e424f081a59738ece0',
    time: 1493939146,
    medianTime: 1493938638,
    nonce: 516909587,
    bits: '1d0135a4',
    difficulty: 0.8267522833930464
},
{
    hash: '000000009ffc1f3135d785e7fb749df33ee630091519da3adfbe89d65adeae09',
    version: 536870912,
    confirmations: 19,
    height: 195472,
    chainWork: '00000000000000000000000000000000000000000000000000005685ee34cb60',
    prevHash: '00000000ca292082e7175f3d6f6862207611539ffd5017611c57373803d16a7d',
    nextHash: '00000000a68fd72c496ab5879e38efecd8dc57dd5890785e4ec4602555e559db',
    merkleRoot: 'dd967c44432a3a67e7edc849bcd066d96a9948f25bda33bcaad1128a534cb4a4',
    time: 1493939298,
    medianTime: 1493938915,
    nonce: 213906176,
    bits: '1d013879',
    difficulty: 0.8192591851787031
},
{
    hash: '00000000a68fd72c496ab5879e38efecd8dc57dd5890785e4ec4602555e559db',
    version: 536870912,
    confirmations: 18,
    height: 195473,
    chainWork: '00000000000000000000000000000000000000000000000000005686cd0bf6c3',
    prevHash: '000000009ffc1f3135d785e7fb749df33ee630091519da3adfbe89d65adeae09',
    nextHash: '0000000062fd829ccfe6576feb9b03efcf585256ba56fa252f25ec5921f44183',
    merkleRoot: 'fc388372272dfd3b0eeed8b1197ae0020bee8150c01dc6e96d525fdd5984d484',
    time: 1493939576,
    medianTime: 1493938970,
    nonce: 2004747272,
    bits: '1d012618',
    difficulty: 0.8704574434172776
},
{
    hash: '0000000062fd829ccfe6576feb9b03efcf585256ba56fa252f25ec5921f44183',
    version: 536870912,
    confirmations: 17,
    height: 195474,
    chainWork: '00000000000000000000000000000000000000000000000000005687a5b9b6d6',
    prevHash: '00000000a68fd72c496ab5879e38efecd8dc57dd5890785e4ec4602555e559db',
    nextHash: '000000011b92a4ae6a8ff583270e187280b56fcc8708b87cae2fadb25d6e4b6e',
    merkleRoot: 'ce65dd647690b2ce954366cd61ab19cab7bcd573f0f75a870cc8597d91547ecb',
    time: 1493940198,
    medianTime: 1493939019,
    nonce: 966443286,
    bits: '1d012e75',
    difficulty: 0.846388304123778
},
{
    hash: '000000011b92a4ae6a8ff583270e187280b56fcc8708b87cae2fadb25d6e4b6e',
    version: 536870912,
    confirmations: 16,
    height: 195475,
    chainWork: '000000000000000000000000000000000000000000000000000056886804c49c',
    prevHash: '0000000062fd829ccfe6576feb9b03efcf585256ba56fa252f25ec5921f44183',
    nextHash: '000000003454e5aa349ab1aa75b7760f6e40a661aa8bf8a804daf263aaa9063c',
    merkleRoot: '10b66bc6b222858623ed63640b295b59bc7f0049bc5f9326887cc07ea5344ccf',
    time: 1493940411,
    medianTime: 1493939078,
    nonce: 3105708302,
    bits: '1d01514e',
    difficulty: 0.7589461493920092
},
{
    hash: '000000003454e5aa349ab1aa75b7760f6e40a661aa8bf8a804daf263aaa9063c',
    version: 536870912,
    confirmations: 15,
    height: 195476,
    chainWork: '000000000000000000000000000000000000000000000000000056892fb155b8',
    prevHash: '000000011b92a4ae6a8ff583270e187280b56fcc8708b87cae2fadb25d6e4b6e',
    nextHash: '00000000bbc3d6cac077ab6eefaa3f5dca657d9607fb75864c4d106c4e004046',
    merkleRoot: '74538dd4070fda203e16d6601bf698a219d48fcfab846df54fab0a8ba06d0673',
    time: 1493940607,
    medianTime: 1493939146,
    nonce: 418067477,
    bits: '1d014837',
    difficulty: 0.77996500958071
},
{
    hash: '00000000bbc3d6cac077ab6eefaa3f5dca657d9607fb75864c4d106c4e004046',
    version: 536870912,
    confirmations: 14,
    height: 195477,
    chainWork: '00000000000000000000000000000000000000000000000000005689f1f7347c',
    prevHash: '000000003454e5aa349ab1aa75b7760f6e40a661aa8bf8a804daf263aaa9063c',
    nextHash: '000000002f496b9d082eb1ef5890dfc00b0a69fa75943f9b7c23a04092e5b95d',
    merkleRoot: 'c839511229aa163614dbe979f18fcfb8ff86539597c9ccb90de8b26d42a0f777',
    time: 1493940652,
    medianTime: 1493939298,
    nonce: 2413602568,
    bits: '1d015157',
    difficulty: 0.758867054968214
},
{
    hash: '000000002f496b9d082eb1ef5890dfc00b0a69fa75943f9b7c23a04092e5b95d',
    version: 536870912,
    confirmations: 13,
    height: 195478,
    chainWork: '0000000000000000000000000000000000000000000000000000568ab0830b5c',
    prevHash: '00000000bbc3d6cac077ab6eefaa3f5dca657d9607fb75864c4d106c4e004046',
    nextHash: '000000008b7ea77d2c6a0b416369c449e2ada6c3f65cb16192f19c9f7f31a7dc',
    merkleRoot: '2caae989922eb54de98a3e15e21e6d2fbbf55a014117657652ac99ce11e9adbb',
    time: 1493940670,
    medianTime: 1493939576,
    nonce: 1453632784,
    bits: '1d0157f0',
    difficulty: 0.7443099218608032
},
{
    hash: '000000008b7ea77d2c6a0b416369c449e2ada6c3f65cb16192f19c9f7f31a7dc',
    version: 536870912,
    confirmations: 12,
    height: 195479,
    chainWork: '0000000000000000000000000000000000000000000000000000568b7054969c',
    prevHash: '000000002f496b9d082eb1ef5890dfc00b0a69fa75943f9b7c23a04092e5b95d',
    nextHash: '000000014d12f6703e811cdb2df877d45509acbe1183970dc6276d8efac084de',
    merkleRoot: '9be43e429835f737b16de57ce6710c48b54ecb157ef791e120b991b64c9d7cd8',
    time: 1493940814,
    medianTime: 1493940198,
    nonce: 3003733250,
    bits: '1d0155a8',
    difficulty: 0.7492797036495015
},
{
    hash: '000000014d12f6703e811cdb2df877d45509acbe1183970dc6276d8efac084de',
    version: 536870912,
    confirmations: 11,
    height: 195480,
    chainWork: '0000000000000000000000000000000000000000000000000000568c28f05a08',
    prevHash: '000000008b7ea77d2c6a0b416369c449e2ada6c3f65cb16192f19c9f7f31a7dc',
    nextHash: '00000000e13979f7b62f80f2e596b81b840c90ddb8159b2b9eb5005d4f2c6d5e',
    merkleRoot: '59033244041e2c8698f595b58f33711cb681d86292ce99b79e5e3e4b4720bffe',
    time: 1493940875,
    medianTime: 1493940411,
    nonce: 4125913101,
    bits: '1d016300',
    difficulty: 0.7211157570422535
},
{
    hash: '00000000e13979f7b62f80f2e596b81b840c90ddb8159b2b9eb5005d4f2c6d5e',
    version: 536870912,
    confirmations: 10,
    height: 195481,
    chainWork: '0000000000000000000000000000000000000000000000000000568ce3905b4e',
    prevHash: '000000014d12f6703e811cdb2df877d45509acbe1183970dc6276d8efac084de',
    nextHash: '00000000868b5612857f20e505226d5ad02d03535dba4f14a9f55e11c209ea3b',
    merkleRoot: 'abe2fe0208035edad87b15eec5f53979473717706562ae67fed516a0cc6aeeb7',
    time: 1493941344,
    medianTime: 1493940607,
    nonce: 331545880,
    bits: '1d015f2a',
    difficulty: 0.7289928585730494
},
{
    hash: '00000000868b5612857f20e505226d5ad02d03535dba4f14a9f55e11c209ea3b',
    version: 536870912,
    confirmations: 9,
    height: 195482,
    chainWork: '0000000000000000000000000000000000000000000000000000568d8b1a6995',
    prevHash: '00000000e13979f7b62f80f2e596b81b840c90ddb8159b2b9eb5005d4f2c6d5e',
    nextHash: '0000000067be3bd5ada7b8b035a8ba173e3a4f435b11e9cc54869f0c541053ef',
    merkleRoot: '78a5c61ab94b1f635fbff7491433c5eeb9de1be03582c2578f3289a4758b98b8',
    time: 1493941506,
    medianTime: 1493940652,
    nonce: 612070147,
    bits: '1d01872b',
    difficulty: 0.6544403279441576
},
{
    hash: '0000000067be3bd5ada7b8b035a8ba173e3a4f435b11e9cc54869f0c541053ef',
    version: 536870912,
    confirmations: 8,
    height: 195483,
    chainWork: '0000000000000000000000000000000000000000000000000000568e2f53fcc4',
    prevHash: '00000000868b5612857f20e505226d5ad02d03535dba4f14a9f55e11c209ea3b',
    nextHash: '00000001132ef80db82698651cf16f7bc24fe82dd0440598c439d01f70f021f4',
    merkleRoot: '6fc706696864085f3f110a707bc4b9a665484ef1a3d796114afbfab0a6aee390',
    time: 1493941845,
    medianTime: 1493940670,
    nonce: 2953139479,
    bits: '1d018f10',
    difficulty: 0.6414937353171496
},
{
    hash: '00000001132ef80db82698651cf16f7bc24fe82dd0440598c439d01f70f021f4',
    version: 536870912,
    confirmations: 7,
    height: 195484,
    chainWork: '0000000000000000000000000000000000000000000000000000568ec9e9449c',
    prevHash: '0000000067be3bd5ada7b8b035a8ba173e3a4f435b11e9cc54869f0c541053ef',
    nextHash: '000000009aaa0d2c6c104ddf375f786b4da9d7087024bd08887001d42f7a0525',
    merkleRoot: 'aeb01b947a075a6906e355abfcd132b5b125926393628f2f9a692b1c76936eee',
    time: 1493941893,
    medianTime: 1493940814,
    nonce: 3942214401,
    bits: '1d01a7f4',
    difficulty: 0.6038311281465374
}];