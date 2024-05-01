const getDocumentTransitionsFixture = require('../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const { default: loadWasmDpp } = require('../../../../../../dist');

let DataContract;
let DocumentCreateTransition;

describe.skip('DocumentCreateTransition', () => {
  let documentTransitionJs;
  let documentTransition;

  beforeEach(async () => {
    ({
      DataContract, DocumentCreateTransition,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    [documentTransitionJs] = getDocumentTransitionsFixture();
    const dataContractJs = documentTransitionJs.dataContract;
    const dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentTransition = new DocumentCreateTransition(
      documentTransitionJs.toObject(),
      dataContract,
    );
  });

  describe('toJSON', () => {
    it('should return json representation', () => {
      const jsonDocumentTransition = documentTransitionJs.toJSON();

      expect(jsonDocumentTransition).to.deep.equal({
        $id: documentTransitionJs.getId().toString(),
        $type: documentTransitionJs.getType(),
        $action: documentTransitionJs.getAction(),
        $dataContractId: documentTransitionJs.getDataContractId().toString(),
        $entropy: documentTransitionJs.getEntropy().toString('base64'),
        $createdAt: documentTransitionJs.getCreatedAt().getTime(),
        name: documentTransitionJs.getData().name,
      });
    });

    it('should return json representation - Rust', () => {
      const jsonDocumentTransition = documentTransition.toJSON();

      expect(jsonDocumentTransition).to.deep.equal({
        $id: documentTransitionJs.getId().toString(),
        $type: documentTransitionJs.getType(),
        $action: documentTransitionJs.getAction(),
        $dataContractId: documentTransitionJs.getDataContractId().toString(),
        $entropy: documentTransitionJs.getEntropy().toString('base64'),
        $createdAt: documentTransitionJs.getCreatedAt().getTime(),
        name: documentTransitionJs.getData().name,
      });
    });
  });
});
