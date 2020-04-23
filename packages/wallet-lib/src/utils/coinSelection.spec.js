const { expect } = require('chai');
const { Transaction } = require('@dashevo/dashcore-lib');
const coinSelection = require('./coinSelection');
const { utxosList } = require('../../fixtures/crackspice');
const STRATEGIES = require('./coinSelections/strategies');

const outputs = {
  ONE_DASH: {
    satoshis: 100000000,
    address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
  },
  HUNDRED_DASH: {
    satoshis: 10000000000,
    address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
  },
  TWENTY_FIVE_DASH: {
    satoshis: 2500000000,
    address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
  },
  FOURTY_FIVE_DASH: {
    satoshis: 4500000000,
    address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
  },
  MILLION_DASH: {
    satoshis: 100000000000000,
    address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
  },
};
describe('Utils - coinSelection', () => {
  it('should require a utxosList', () => {
    expect(() => coinSelection()).to.throw('A utxosList is required');
  });
  it('should require a utxosList as an array', () => {
    expect(() => coinSelection({})).to.throw('UtxosList is expected to be an array of utxos');
  });
  it('should require a utxosList with at least one utxo', () => {
    expect(() => coinSelection([])).to.throw('utxosList must contain at least 1 utxo');
  });

  it('should require a utxosList with valid utxo', () => {
    expect(() => coinSelection([{
      toto: true,
    }])).to.throw('UTXO txid:unknown should have property txid of type txid');
  });


  it('should require a outputsList', () => {
    expect(() => coinSelection(utxosList)).to.throw('An outputsList is required in order to perform a selection');
  });
  // return;
  it('should require a outputsList as an array', () => {
    expect(() => coinSelection(utxosList, {})).to.throw('outputsList must be an array of outputs');
  });
  it('should require a outputsList with at least one output', () => {
    expect(() => coinSelection(utxosList, [])).to.throw('outputsList must contains at least 1 output');
  });

  it('should require a outputsList with valid outputs', () => {
    expect(() => coinSelection(utxosList, [{ toto: true }])).to.throw('Output txid:unknown address: unknown should have property address of type string');
  });
  it('should alert if the total amount is not enough', () => {
    expect(() => coinSelection(utxosList, [outputs.HUNDRED_DASH])).to.throw('Unsufficient utxos (7099960000) to cover the output : 10000000000. Diff : -2900040000');
  });
  it('should work with normal utxo format', () => {
    const output = new Transaction.UnspentOutput({
      address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
      txid: 'd928aedc4ecc6c251cabee0672c19308573e5b4898c32779f3fd211dd8a1fbd8',
      outputIndex: 1,
      script: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
      satoshis: 12999997288,
    });
    console.log(output.script.toAddress('testnet').toString());
    const result = coinSelection([output], [{
      satoshis: 2999997288,
      address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
    }]);
    const expectedResult = {
      utxos: [output],
      outputs: [{ satoshis: 2999997288, address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7', scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 12999997288,
    };

    expect(result).to.deep.equal(expectedResult);
  });
  it('should get a coinSelection for 1 dash', () => {
    const result = coinSelection(utxosList, [outputs.ONE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [{
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        amount: 1,
        txid: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
      }],
      outputs: [{ satoshis: 100000000, address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7', scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 100000000,
    };
    result.utxos = result.utxos.map((el) => el.toJSON());
    expect(result).to.deep.equal(expectedResult);
  });
  it('should handle a case when using more than 25 utxos', () => {
    const result = coinSelection(utxosList, [outputs.TWENTY_FIVE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [{
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '0c98713b9895cf6c48f15aa717561f78339b9701f927c057758cb617f671cbfd',
        vout: 0,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '1240c9e3bba3f143ec354bd37e4b860609b944dee2e426e9868e5c3244e47f04',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 0.8,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '157a4869ac5de33f40812f1e50e50395b472f991a72e59170037671914e72b0d',
        vout: 1,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '1d90ba700b8fa18c8d9a6d3eaa505dde99a4a459c0d1e73bf40ba4b2cc2461cc',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '1fe685297c8c188a440affdda538ef5c757399051965352157c7e1495e6038f0',
        vout: 1,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL',
        txid: '22c368e09ad8b36553b383c6a4ae989f91d1f66622b2b685262580c8a45175a4',
        vout: 1,
        scriptPubKey: '76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac',
        amount: 0.5,
      }, {
        address: 'yZruigeCbPHVRnJG9JcSyG9AhX7PSF9oi7',
        txid: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        vout: 0,
        scriptPubKey: '76a914948cf5d360500a04d0a9080eac8514b79c1297b288ac',
        amount: 1.5,
      }, {
        address: 'yPWVEG3mW8pFdPCXcE53gN1fSTM8dkV7kF',
        txid: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        vout: 1,
        scriptPubKey: '76a91422fef09d745700a159553dd42227895053d33e6888ac',
        amount: 8.4999,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '2bf25390be738308827348711da2700918b73096bfaff99de6c9c60121fa5d8e',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 2,
      }, {
        address: 'yNgqjoW69ouSivtBMNFRCG5zSG85nyxW3d',
        txid: '36820d7268090d6f315eef03b28b7b2b2097c8b067608f652612a2c4612a6697',
        vout: 1,
        scriptPubKey: '76a91419fc1815a04c42a849a7a6dda826c67478514fed88ac',
        amount: 9.9999,
      }],
      outputs: [{ satoshis: 2500000000, address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7', scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 1655,
      utxosValue: 2829980000,
    };
    result.utxos = result.utxos.map((el) => el.toObject());
    expect(result).to.deep.equal(expectedResult);
  });
  it('should handle a case when using more than 45 utxos', () => {
    const result = coinSelection(utxosList, [outputs.FOURTY_FIVE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [{
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '0c98713b9895cf6c48f15aa717561f78339b9701f927c057758cb617f671cbfd',
        vout: 0,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '1240c9e3bba3f143ec354bd37e4b860609b944dee2e426e9868e5c3244e47f04',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 0.8,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '157a4869ac5de33f40812f1e50e50395b472f991a72e59170037671914e72b0d',
        vout: 1,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '1d90ba700b8fa18c8d9a6d3eaa505dde99a4a459c0d1e73bf40ba4b2cc2461cc',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '1fe685297c8c188a440affdda538ef5c757399051965352157c7e1495e6038f0',
        vout: 1,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL',
        txid: '22c368e09ad8b36553b383c6a4ae989f91d1f66622b2b685262580c8a45175a4',
        vout: 1,
        scriptPubKey: '76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac',
        amount: 0.5,
      }, {
        address: 'yZruigeCbPHVRnJG9JcSyG9AhX7PSF9oi7',
        txid: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        vout: 0,
        scriptPubKey: '76a914948cf5d360500a04d0a9080eac8514b79c1297b288ac',
        amount: 1.5,
      }, {
        address: 'yPWVEG3mW8pFdPCXcE53gN1fSTM8dkV7kF',
        txid: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        vout: 1,
        scriptPubKey: '76a91422fef09d745700a159553dd42227895053d33e6888ac',
        amount: 8.4999,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '2bf25390be738308827348711da2700918b73096bfaff99de6c9c60121fa5d8e',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 2,
      }, {
        address: 'yNgqjoW69ouSivtBMNFRCG5zSG85nyxW3d',
        txid: '36820d7268090d6f315eef03b28b7b2b2097c8b067608f652612a2c4612a6697',
        vout: 1,
        scriptPubKey: '76a91419fc1815a04c42a849a7a6dda826c67478514fed88ac',
        amount: 9.9999,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '5b14eea2e1e07f94fbce22b50b6cda6b748a66c1119524a623c6820b75bbc7ca',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 5,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '5b6efaffbcf24b613ce29e18263203e05406f3fc130377eac02d579964672d67',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '5c462466bea61ff28e7805d20b482d83a139ea300a76052921038a22705e6937',
        vout: 0,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 2,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '6c42619dd84a02577458ba4f880fe8cfaced9ed518ee7c360c5b107d6ff5b62d',
        vout: 0,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 1,
      }, {
        address: 'yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX',
        txid: '7a6578995dd6eb11f0ec08e61135363fab55c0732ac05f563088b864d62f8cd4',
        vout: 1,
        scriptPubKey: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        amount: 1,
      }, {
        address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
        txid: '8053a30d671b62e56a4a61d1fe2f899917cd20278e474a433e8d88d140757e0e',
        vout: 1,
        scriptPubKey: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
        amount: 2,
      }, {
        address: 'yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL',
        txid: '96eb6c951d69a3b8703673ca0d588cf6cee528f866fc598e84205ddcc34ea100',
        vout: 0,
        scriptPubKey: '76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac',
        amount: 2,
      }, {
        address: 'yPn5VvPk7ioN9emDv3MkCKovpjNqSLwW1p',
        txid: '96eb6c951d69a3b8703673ca0d588cf6cee528f866fc598e84205ddcc34ea100',
        vout: 1,
        scriptPubKey: '76a91425f1c9581cd2a9976e6ace867f8e895663e6825a88ac',
        amount: 6.9998,
      }],
      outputs: [{ satoshis: 4500000000, address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7', scriptType: 'P2PKH' }],
      feeCategory: 'normal',
      estimatedFee: 2815,
      utxosValue: 4929960000,
    };
    result.utxos = result.utxos.map((el) => el.toObject());

    expect(result).to.deep.equal(expectedResult);
  });
  // Note : Removed, kept in case of fallback needed
  // it('should return an error in not any utxo has been found', () => {
  //   const utxo = utxosList[15];
  //   const utxos = [];
  //   for (let i = 0; i <= 45; i++) {
  //     utxos.push(utxosList[15]);
  //   }
  // expect(() => coinSelection(utxos, [outputs.FOURTY_FIVE_DASH])).to.throw('Did not found any utxo, missing implementation of this case');
  // });
});
