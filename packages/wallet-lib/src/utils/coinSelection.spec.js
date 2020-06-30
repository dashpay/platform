const {expect} = require('chai');
const {Transaction, Address, Script} = require('@dashevo/dashcore-lib');
const coinSelection = require('./coinSelection');
const {utxosList} = require('../../fixtures/crackspice');
const STRATEGIES = require('./coinSelections/strategies');
const TransactionEstimator = require('./coinSelections/TransactionEstimator')

const utxosListAsUnspentOutput = utxosList.map((utxo)=> Transaction.UnspentOutput(utxo));
const outputs = {
  ONE_DASH: {
    satoshis: 100000000,
    address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'),
  },
  HUNDRED_DASH: {
    satoshis: 10000000000,
    address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'),
  },
  TWENTY_FIVE_DASH: {
    satoshis: 2500000000,
    address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'),
  },
  FOURTY_FIVE_DASH: {
    satoshis: 4500000000,
    address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'),
  },
  MILLION_DASH: {
    satoshis: 100000000000000,
    address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'),
  },
};
describe('Utils - coinSelection', function suite() {
  this.timeout(10000);
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
    }])).to.throw('An outputsList is required in order to perform a selection');
  });


  it('should require a outputsList', () => {
    expect(() => coinSelection(utxosList)).to.throw('An outputsList is required in order to perform a selection');
  });
  // return;
  it('should require a outputsList as an array', () => {
    expect(() => coinSelection(utxosListAsUnspentOutput, {})).to.throw('outputsList must be an array of outputs');
  });
  it('should require a outputsList with at least one output', () => {
    expect(() => coinSelection(utxosList, [])).to.throw('outputsList must contains at least 1 output');
  });
  it('should require a outputsList with valid outputs', () => {
    expect(() => coinSelection(utxosListAsUnspentOutput, [{toto: true}])).to.throw('data parameter supplied is not a string.');
  });
  it('should alert if the total satoshis is not enough', () => {
    expect(() => coinSelection(utxosListAsUnspentOutput, [outputs.HUNDRED_DASH])).to.throw('Unsufficient utxos (7099960000) to cover the output : 10000000000. Diff : -2900040000');
  });
  it('should work with normal utxo format', () => {
    const output = new Transaction.UnspentOutput({
      address: 'yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42',
      txId: 'd928aedc4ecc6c251cabee0672c19308573e5b4898c32779f3fd211dd8a1fbd8',
      outputIndex: 1,
      script: '76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac',
      satoshis: 12999997288,
    });

    const result = coinSelection([output], [{
      satoshis: 2999997288,
      address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7',
    }]);
    const expectedResult = {
      utxos: [output],
      outputs: [{satoshis: 2999997288, address: 'ybefxSHaEbDATvq5gVCxjV375NWus3ttV7', scriptType: 'P2PKH'}],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 12999997288,
    };

    expect(result).to.deep.equal(expectedResult);
  });
  it('should get a coinSelection for 1 dash', () => {
    const result = coinSelection(utxosListAsUnspentOutput, [outputs.ONE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        satoshis: 1*1e8,
        txId: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        outputIndex: 1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
      })],
      outputs: [{satoshis: 100000000, address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'), scriptType: 'P2PKH'}],
      feeCategory: 'normal',
      estimatedFee: 205,
      utxosValue: 100000000,
    };
    expect(result).to.deep.equal(expectedResult);
  });
  it('should handle a case when using more than 25 utxos', () => {
    const result = coinSelection(utxosListAsUnspentOutput, [outputs.TWENTY_FIVE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [
          new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX','testnet'),
        txId: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        outputIndex: 1,
        height: 203268,
        amount:1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 1 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '0c98713b9895cf6c48f15aa717561f78339b9701f927c057758cb617f671cbfd',
        outputIndex: 0,
        height: 203265,
        amount:1,
        script: '76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac',
        satoshis: 1 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '1240c9e3bba3f143ec354bd37e4b860609b944dee2e426e9868e5c3244e47f04',
        outputIndex: 1,
        height: 203207,
        amount: 0.8,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 0.8 * 1e8,
      }), new Transaction.UnspentOutput( {
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '157a4869ac5de33f40812f1e50e50395b472f991a72e59170037671914e72b0d',
        outputIndex: 1,
        height:203277,
        amount: 1,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '1d90ba700b8fa18c8d9a6d3eaa505dde99a4a459c0d1e73bf40ba4b2cc2461cc',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput( {
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '1fe685297c8c188a440affdda538ef5c757399051965352157c7e1495e6038f0',
        outputIndex: 1,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL'),
        txId: '22c368e09ad8b36553b383c6a4ae989f91d1f66622b2b685262580c8a45175a4',
        outputIndex: 1,
        script: new Script('76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac'),
        satoshis: 0.5 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yZruigeCbPHVRnJG9JcSyG9AhX7PSF9oi7'),
        txId: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        outputIndex: 0,
        script: new Script('76a914948cf5d360500a04d0a9080eac8514b79c1297b288ac'),
        satoshis: 1.5 * 1e8,
      }), new Transaction.UnspentOutput( {
        address: new Address('yPWVEG3mW8pFdPCXcE53gN1fSTM8dkV7kF'),
        txId: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        outputIndex: 1,
        script: new Script('76a91422fef09d745700a159553dd42227895053d33e6888ac'),
        satoshis: 8.4999 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '2bf25390be738308827348711da2700918b73096bfaff99de6c9c60121fa5d8e',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 2 * 1e8,
      }),  new Transaction.UnspentOutput({
        address: new Address('yNgqjoW69ouSivtBMNFRCG5zSG85nyxW3d'),
        txId: '36820d7268090d6f315eef03b28b7b2b2097c8b067608f652612a2c4612a6697',
        outputIndex: 1,
        script: new Script('76a91419fc1815a04c42a849a7a6dda826c67478514fed88ac'),
        satoshis: 9.9999 * 1e8,
      })],
      outputs: [{satoshis: 2500000000, address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'), scriptType: 'P2PKH'}],
      feeCategory: 'normal',
      estimatedFee: 1655,
      utxosValue: 2829980000,
    };
    expect(result).to.deep.equal(expectedResult);
  });
  it('should handle a case when using more than 45 utxos', () => {
    const result = coinSelection(utxosListAsUnspentOutput, [outputs.FOURTY_FIVE_DASH], false, 'normal', STRATEGIES.simpleDescendingAccumulator);
    const expectedResult = {
      utxos: [
          new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '071502a8b211e08f575641f3345b687a86c922108b5fd608822bffe0151aaf09',
        outputIndex: 1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '0c98713b9895cf6c48f15aa717561f78339b9701f927c057758cb617f671cbfd',
        outputIndex: 0,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '1240c9e3bba3f143ec354bd37e4b860609b944dee2e426e9868e5c3244e47f04',
        outputIndex: 1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 0.8 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '157a4869ac5de33f40812f1e50e50395b472f991a72e59170037671914e72b0d',
        outputIndex: 1,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '1d90ba700b8fa18c8d9a6d3eaa505dde99a4a459c0d1e73bf40ba4b2cc2461cc',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '1fe685297c8c188a440affdda538ef5c757399051965352157c7e1495e6038f0',
        outputIndex: 1,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL'),
        txId: '22c368e09ad8b36553b383c6a4ae989f91d1f66622b2b685262580c8a45175a4',
        outputIndex: 1,
        script: new Script('76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac'),
        satoshis: 0.5 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yZruigeCbPHVRnJG9JcSyG9AhX7PSF9oi7'),
        txId: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        outputIndex: 0,
        script: new Script('76a914948cf5d360500a04d0a9080eac8514b79c1297b288ac'),
        satoshis: 1.5 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yPWVEG3mW8pFdPCXcE53gN1fSTM8dkV7kF'),
        txId: '2911362650f08df1ea16e03973bb41e1ee33680cce2ec6ce864e2daf35431e08',
        outputIndex: 1,
        script: new Script('76a91422fef09d745700a159553dd42227895053d33e6888ac'),
        satoshis: 8.4999 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '2bf25390be738308827348711da2700918b73096bfaff99de6c9c60121fa5d8e',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 2 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yNgqjoW69ouSivtBMNFRCG5zSG85nyxW3d'),
        txId: '36820d7268090d6f315eef03b28b7b2b2097c8b067608f652612a2c4612a6697',
        outputIndex: 1,
        script: new Script('76a91419fc1815a04c42a849a7a6dda826c67478514fed88ac'),
        satoshis: 9.9999 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '5b14eea2e1e07f94fbce22b50b6cda6b748a66c1119524a623c6820b75bbc7ca',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 5 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '5b6efaffbcf24b613ce29e18263203e05406f3fc130377eac02d579964672d67',
        outputIndex: 1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '5c462466bea61ff28e7805d20b482d83a139ea300a76052921038a22705e6937',
        outputIndex: 0,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 2 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '6c42619dd84a02577458ba4f880fe8cfaced9ed518ee7c360c5b107d6ff5b62d',
        outputIndex: 0,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yQeCpWLJNGP4Aiojmz5ZC5gbYXREsnLnaX'),
        txId: '7a6578995dd6eb11f0ec08e61135363fab55c0732ac05f563088b864d62f8cd4',
        outputIndex: 1,
        script: new Script('76a9142f6cb2047c14f0068a561fa2df704e64467ce9c588ac'),
        satoshis: 1 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yW5qRPWdp1NzvxPbE4v95FDCxjxNqDEi42'),
        txId: '8053a30d671b62e56a4a61d1fe2f899917cd20278e474a433e8d88d140757e0e',
        outputIndex: 1,
        script: new Script('76a9146b1e46d3f3d559dda4468cc30a7b612705eb810f88ac'),
        satoshis: 2 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yMfDnWF6piqNA7mbSeEeAP4LiiqgxkJvNL'),
        txId: '96eb6c951d69a3b8703673ca0d588cf6cee528f866fc598e84205ddcc34ea100',
        outputIndex: 0,
        script: new Script('76a9140eb58a39a96968c19411568752ecdecf55dabb8588ac'),
        satoshis: 2 * 1e8,
      }), new Transaction.UnspentOutput({
        address: new Address('yPn5VvPk7ioN9emDv3MkCKovpjNqSLwW1p'),
        txId: '96eb6c951d69a3b8703673ca0d588cf6cee528f866fc598e84205ddcc34ea100',
        outputIndex: 1,
        script: new Script('76a91425f1c9581cd2a9976e6ace867f8e895663e6825a88ac'),
        satoshis: 6.9998 * 1e8,
      })],
      outputs: [{satoshis: 4500000000, address: new Address('ybefxSHaEbDATvq5gVCxjV375NWus3ttV7'), scriptType: 'P2PKH'}],
      feeCategory: 'normal',
      estimatedFee: 2815,
      utxosValue: 4929960000,
    };
    // result.utxos = result.utxos.map((el) => el.toObject());

    expect(result).to.deep.equal(expectedResult);
  });
  it('should handle externally crafted strategy', function () {
    // A dummy strategy that takes a random selection of utxo
    const externalStrategy = (utxosList, outputsList, deductFee = false, feeCategory = 'normal') => {
      const copiedUtxos = [...utxosList];
      const txEstimator = new TransactionEstimator(feeCategory);

      txEstimator.addOutputs(outputsList);

      let inputValue = 0;
      let outputValue = txEstimator.getOutValue();
      const randomlySelectedUtxos = [];

      while(inputValue<outputValue){
        if(copiedUtxos.length === 0){
          throw new Error('Not enought UTXOs');
        }
        // Take a random item and add it to selection
        const utxo = copiedUtxos.splice(Math.floor(Math.random() * copiedUtxos.length),1)[0];
        inputValue+=utxo.satoshis;
        randomlySelectedUtxos.push(utxo);
      }

      txEstimator.addInputs(randomlySelectedUtxos);

      return {
        utxos: txEstimator.getInputs(),
        outputs: txEstimator.getOutputs(),
        feeCategory,
        estimatedFee: txEstimator.getFeeEstimate(),
        utxosValue: txEstimator.getInValue(),
      };
    }
    const result = coinSelection(
        utxosListAsUnspentOutput,
        [outputs.FOURTY_FIVE_DASH],
        false,
        'normal',
        externalStrategy);

    expect(result).to.exist;
    expect(result.feeCategory).to.equal('normal');
    expect(result.utxosValue).to.gte(4500000000);
    expect(result.outputs).to.deep.equal([outputs.FOURTY_FIVE_DASH]);
    expect(result.utxos.length).to.gte(0);
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
