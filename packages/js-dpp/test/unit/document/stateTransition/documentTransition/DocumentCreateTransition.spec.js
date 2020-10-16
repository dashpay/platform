const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

describe('DocumentCreateTransition', () => {
  let documentTransition;

  beforeEach(() => {
    [documentTransition] = getDocumentTransitionsFixture();
  });

  describe('toJSON', () => {
    it('should return json representation', () => {
      const jsonDocumentTransition = documentTransition.toJSON();

      expect(jsonDocumentTransition).to.deep.equal({
        $id: documentTransition.getId().toString(),
        $type: documentTransition.getType(),
        $action: documentTransition.getAction(),
        $dataContractId: documentTransition.getDataContractId().toString(),
        $entropy: documentTransition.getEntropy().toString('base64'),
        $createdAt: documentTransition.getCreatedAt().getTime(),
        name: documentTransition.getData().name,
      });
    });
  });
});
