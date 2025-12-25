import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('FeeStrategyStep', () => {
  describe('construction', () => {
    it('should create deductFromInput step', () => {
      const step = wasm.FeeStrategyStep.deductFromInput(0);
      expect(step).to.exist;
      expect(step.isDeductFromInput).to.be.true;
      expect(step.isReduceOutput).to.be.false;
      expect(step.index).to.equal(0);
    });

    it('should create deductFromInput step with non-zero index', () => {
      const step = wasm.FeeStrategyStep.deductFromInput(5);
      expect(step.isDeductFromInput).to.be.true;
      expect(step.index).to.equal(5);
    });

    it('should create reduceOutput step', () => {
      const step = wasm.FeeStrategyStep.reduceOutput(0);
      expect(step).to.exist;
      expect(step.isDeductFromInput).to.be.false;
      expect(step.isReduceOutput).to.be.true;
      expect(step.index).to.equal(0);
    });

    it('should create reduceOutput step with non-zero index', () => {
      const step = wasm.FeeStrategyStep.reduceOutput(3);
      expect(step.isReduceOutput).to.be.true;
      expect(step.index).to.equal(3);
    });
  });
});

// TODO: Implement AddressFundsTransferTransition in wasm-dpp2
describe.skip('AddressFundsTransferTransition', () => {
  // Valid test private key
  const testPrivateKeyHex = 'c9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd';
  const testPrivateKeyHex2 = 'a9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd';

  describe('build', () => {
    it('should build a transfer transition with single input and output', () => {
      // Create input address
      const inputAddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const inputAddr = wasm.PlatformAddress.fromBytes(Array.from(inputAddrBytes));

      // Create output address
      const outputAddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const outputAddr = wasm.PlatformAddress.fromBytes(Array.from(outputAddrBytes));

      // Create input and output
      const input = new wasm.PlatformAddressInput(inputAddr, BigInt(0), BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      // Create signer
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      signer.addKey(inputAddr, privateKey);

      // Build transition
      const transition = wasm.AddressFundsTransferTransition.build({
        inputs: [input],
        outputs: [output],
        signer,
      });

      expect(transition).to.exist;
      expect(transition.__wbg_ptr).to.not.equal(0);
    });

    it('should build a transfer transition with multiple inputs and outputs', () => {
      // Create input addresses
      const input1AddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const input1Addr = wasm.PlatformAddress.fromBytes(Array.from(input1AddrBytes));

      const input2AddrBytes = new Uint8Array([0x00, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40]);
      const input2Addr = wasm.PlatformAddress.fromBytes(Array.from(input2AddrBytes));

      // Create output addresses
      const output1AddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const output1Addr = wasm.PlatformAddress.fromBytes(Array.from(output1AddrBytes));

      const output2AddrBytes = new Uint8Array([0x00, 40, 39, 38, 37, 36, 35, 34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21]);
      const output2Addr = wasm.PlatformAddress.fromBytes(Array.from(output2AddrBytes));

      // Create inputs
      const input1 = new wasm.PlatformAddressInput(input1Addr, BigInt(0), BigInt(100000));
      const input2 = new wasm.PlatformAddressInput(input2Addr, BigInt(0), BigInt(50000));

      // Create outputs
      const output1 = new wasm.PlatformAddressOutput(output1Addr, BigInt(80000));
      const output2 = new wasm.PlatformAddressOutput(output2Addr, BigInt(60000));

      // Create signer with keys for both inputs
      const signer = new wasm.PlatformAddressSigner();
      const privateKey1 = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      const privateKey2 = wasm.PrivateKey.fromHex(testPrivateKeyHex2, 'testnet');
      signer.addKey(input1Addr, privateKey1);
      signer.addKey(input2Addr, privateKey2);

      // Build transition
      const transition = wasm.AddressFundsTransferTransition.build({
        inputs: [input1, input2],
        outputs: [output1, output2],
        signer,
      });

      expect(transition).to.exist;
    });

    it('should build with custom fee strategy', () => {
      const inputAddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const inputAddr = wasm.PlatformAddress.fromBytes(Array.from(inputAddrBytes));

      const outputAddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const outputAddr = wasm.PlatformAddress.fromBytes(Array.from(outputAddrBytes));

      const input = new wasm.PlatformAddressInput(inputAddr, BigInt(0), BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(100000));

      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      signer.addKey(inputAddr, privateKey);

      // Use reduceOutput fee strategy
      const feeStrategy = [wasm.FeeStrategyStep.reduceOutput(0)];

      const transition = wasm.AddressFundsTransferTransition.build({
        inputs: [input],
        outputs: [output],
        signer,
        feeStrategy,
      });

      expect(transition).to.exist;
    });

    it('should build with user fee increase', () => {
      const inputAddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const inputAddr = wasm.PlatformAddress.fromBytes(Array.from(inputAddrBytes));

      const outputAddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const outputAddr = wasm.PlatformAddress.fromBytes(Array.from(outputAddrBytes));

      const input = new wasm.PlatformAddressInput(inputAddr, BigInt(0), BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      signer.addKey(inputAddr, privateKey);

      const transition = wasm.AddressFundsTransferTransition.build({
        inputs: [input],
        outputs: [output],
        signer,
        userFeeIncrease: 100,
      });

      expect(transition).to.exist;
    });

    it('should fail without inputs', () => {
      const outputAddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const outputAddr = wasm.PlatformAddress.fromBytes(Array.from(outputAddrBytes));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      const signer = new wasm.PlatformAddressSigner();

      try {
        wasm.AddressFundsTransferTransition.build({
          inputs: [],
          outputs: [output],
          signer,
        });
        expect.fail('Should have thrown error for missing inputs');
      } catch (error) {
        expect(error.message).to.include('input');
      }
    });

    it('should fail without outputs', () => {
      const inputAddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const inputAddr = wasm.PlatformAddress.fromBytes(Array.from(inputAddrBytes));
      const input = new wasm.PlatformAddressInput(inputAddr, BigInt(0), BigInt(100000));

      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      signer.addKey(inputAddr, privateKey);

      try {
        wasm.AddressFundsTransferTransition.build({
          inputs: [input],
          outputs: [],
          signer,
        });
        expect.fail('Should have thrown error for missing outputs');
      } catch (error) {
        expect(error.message).to.include('output');
      }
    });

    it('should fail without signer', () => {
      const inputAddrBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const inputAddr = wasm.PlatformAddress.fromBytes(Array.from(inputAddrBytes));
      const input = new wasm.PlatformAddressInput(inputAddr, BigInt(0), BigInt(100000));

      const outputAddrBytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const outputAddr = wasm.PlatformAddress.fromBytes(Array.from(outputAddrBytes));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      try {
        wasm.AddressFundsTransferTransition.build({
          inputs: [input],
          outputs: [output],
        });
        expect.fail('Should have thrown error for missing signer');
      } catch (error) {
        expect(error.message).to.include('signer');
      }
    });
  });
});
